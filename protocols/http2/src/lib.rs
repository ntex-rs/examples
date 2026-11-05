//! A standalone HTTP/2 server and client with ntex-h2.
//!
//! [`Handler`] is the publish service of the server. ntex-h2 calls it for
//! every message of every stream, concurrently, so it keeps per-stream state
//! in a map keyed by [`StreamRef`]. It serves three routes:
//!
//! - `GET /hello` answers with a fixed body.
//! - `POST /echo` buffers the request body, up to [`MAX_ECHO_BODY`], and
//!   sends it back.
//! - `POST /count` streams the request body to a separate task together
//!   with its flow-control [`Capacity`]. The task counts the bytes and
//!   answers with the total in the body and in the [`BYTES_TRAILER`] trailer.
use std::{cell::RefCell, collections::HashMap, convert::Infallible, error::Error};

use ntex::channel::mpsc;
use ntex::http::{HeaderMap, Method, StatusCode, header::HeaderValue};
use ntex::service::{Ctx, Service, ServiceFactory, fn_factory};
use ntex::util::{Bytes, BytesMut};
use ntex::{SharedCfg, time::Seconds};
use ntex_h2::{
    Capacity, Message, MessageKind, OperationError, ServiceConfig, StreamEof, StreamRef,
    client::Client, client::RecvStream,
};

/// Address of the server binary.
pub const ADDR: &str = "127.0.0.1:50060";

/// Largest body that `POST /echo` accepts.
pub const MAX_ECHO_BODY: usize = 64 * 1024;

/// Trailer with the number of bytes counted by `POST /count`.
pub const BYTES_TRAILER: &str = "x-bytes";

/// HTTP/2 settings of every server connection.
pub fn server_cfg() -> SharedCfg {
    SharedCfg::new("H2-SRV")
        .add(
            ServiceConfig::new()
                // requests a client may run at once on one connection
                .set_max_concurrent_streams(100)
                // body data a client may send before it must wait for us,
                // per request and for the whole connection
                .set_initial_window_size(128 * 1024)
                .set_initial_connection_window_size(1024 * 1024)
                .set_ping_timeout(Seconds(30)),
        )
        .into()
}

/// The publish service factory, it creates a [`Handler`] for every connection.
pub fn handler() -> impl ServiceFactory<
    (),
    Message,
    Res = (),
    Error = ntex::error::Error<OperationError>,
    InitError = Infallible,
> {
    fn_factory(async |_: &()| Ok::<_, Infallible>(Handler::default()))
}

/// A request whose body is still arriving.
enum Pending {
    /// `POST /echo`: the body collected so far.
    Echo(BytesMut),
    /// `POST /count`: the channel to the task that counts the body.
    Count(mpsc::Sender<(Bytes, Capacity)>),
}

/// Publish service of one connection.
#[derive(Default)]
pub struct Handler {
    pending: RefCell<HashMap<StreamRef, Pending>>,
}

impl Service<(), Message> for Handler {
    type Res = ();
    type Error = ntex::error::Error<OperationError>;

    async fn call(&self, msg: Message, _: Ctx<'_, Self, ()>) -> Result<(), Self::Error> {
        let Message { stream, kind } = msg;

        match kind {
            MessageKind::Headers { pseudo, eof, .. } => {
                let method = pseudo.method.unwrap_or(Method::GET);
                let path = pseudo.path.unwrap_or_default();
                self.start(stream, &method, &path, eof).await
            }
            MessageKind::Data(data, cap) => {
                self.data(&stream, data, cap);
                Ok(())
            }
            MessageKind::Eof(StreamEof::Data(data, cap)) => {
                self.data(&stream, data, cap);
                self.finish(stream).await
            }
            MessageKind::Eof(StreamEof::Trailers(_)) => self.finish(stream).await,
            // the client reset the stream or the connection failed,
            // forget the request; for `/count` this ends the counting task
            MessageKind::Eof(StreamEof::Error(_)) | MessageKind::Disconnect(_) => {
                self.pending.borrow_mut().remove(&stream);
                Ok(())
            }
        }
    }
}

impl Handler {
    /// Handles the request headers.
    async fn start(
        &self,
        stream: StreamRef,
        method: &Method,
        path: &str,
        eof: bool,
    ) -> Result<(), ntex::error::Error<OperationError>> {
        match (method, path) {
            (&Method::GET, "/hello") => {
                stream.send_response(StatusCode::OK, text_plain(), false)?;
                stream.send_payload("hello world", true).await
            }
            (&Method::POST, "/echo") if eof => {
                stream.send_response(StatusCode::OK, HeaderMap::new(), true)
            }
            (&Method::POST, "/echo") => {
                let pending = Pending::Echo(BytesMut::new());
                self.pending.borrow_mut().insert(stream, pending);
                Ok(())
            }
            (&Method::POST, "/count") => {
                let (tx, rx) = mpsc::channel();
                ntex::rt::spawn(count(stream.clone(), rx));
                if !eof {
                    self.pending.borrow_mut().insert(stream, Pending::Count(tx));
                }
                Ok(())
            }
            _ => stream.send_response(StatusCode::NOT_FOUND, HeaderMap::new(), true),
        }
    }

    /// Handles a chunk of the request body.
    fn data(&self, stream: &StreamRef, data: Bytes, cap: Capacity) {
        let mut pending = self.pending.borrow_mut();
        match pending.get_mut(stream) {
            // `cap` is dropped here, the client may send more right away
            Some(Pending::Echo(body)) => {
                body.extend_from_slice(&data);
                if body.len() > MAX_ECHO_BODY {
                    pending.remove(stream);
                    drop(pending);
                    // answer early, then tell the client to stop sending
                    let _ =
                        stream.send_response(StatusCode::PAYLOAD_TOO_LARGE, HeaderMap::new(), true);
                    stream.reset(ntex_h2::frame::Reason::NO_ERROR);
                }
            }
            // `cap` travels with the data, the client can only send as
            // fast as the counting task releases it
            Some(Pending::Count(tx)) => {
                let _ = tx.send((data, cap));
            }
            None => {}
        }
    }

    /// Handles the end of the request body.
    async fn finish(&self, stream: StreamRef) -> Result<(), ntex::error::Error<OperationError>> {
        let pending = self.pending.borrow_mut().remove(&stream);
        match pending {
            Some(Pending::Echo(body)) => {
                stream.send_response(StatusCode::OK, HeaderMap::new(), false)?;
                stream.send_payload(body.freeze(), true).await
            }
            // dropping the sender closes the channel, the task answers
            Some(Pending::Count(_)) | None => Ok(()),
        }
    }
}

/// Counts the body of a `POST /count` request and answers with the total.
async fn count(stream: StreamRef, rx: mpsc::Receiver<(Bytes, Capacity)>) {
    let mut total = 0;
    while let Some((data, cap)) = rx.recv().await {
        total += data.len();
        // releasing the capacity lets the client send more
        drop(cap);
    }
    if stream.is_reset() {
        return;
    }

    let mut trailers = HeaderMap::new();
    trailers.insert(BYTES_TRAILER.parse().unwrap(), HeaderValue::from(total));
    let result = async {
        stream.send_response(StatusCode::OK, text_plain(), false)?;
        stream
            .send_payload(format!("counted {total} bytes"), false)
            .await?;
        stream.send_trailers(trailers)
    };
    if let Err(err) = result.await {
        log::warn!("{:?}: cannot send response: {err}", stream.id());
    }
}

fn text_plain() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ntex::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain"),
    );
    headers
}

/// Creates a client pool for `addr`.
pub fn connect(addr: impl Into<String>) -> Client {
    Client::builder(addr.into())
        .connect_timeout(Seconds(3))
        .build(SharedCfg::new("H2-CLIENT"))
}

/// A complete response.
#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub trailers: HeaderMap,
}

/// Reads a whole response from `recv`.
pub async fn read_response(recv: RecvStream) -> Result<Response, Box<dyn Error>> {
    let mut res = Response {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: Bytes::new(),
        trailers: HeaderMap::new(),
    };
    let mut body = BytesMut::new();

    while let Some(msg) = recv.recv().await {
        match msg.kind {
            MessageKind::Headers {
                pseudo, headers, ..
            } => {
                res.status = pseudo.status.unwrap_or(StatusCode::OK);
                res.headers = headers;
            }
            MessageKind::Data(data, _) | MessageKind::Eof(StreamEof::Data(data, _)) => {
                body.extend_from_slice(&data);
            }
            MessageKind::Eof(StreamEof::Trailers(trailers)) => res.trailers = trailers,
            MessageKind::Eof(StreamEof::Error(err)) => return Err(err.into()),
            MessageKind::Disconnect(err) => return Err(err.into()),
        }
    }
    res.body = body.freeze();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::server::{TestServer, TestServerBuilder};
    use ntex_h2::server::Server;

    /// Serves `handler()` with the configuration of the binary.
    fn start() -> TestServer {
        TestServerBuilder::new(async |_| Server::new(handler()))
            .config(server_cfg())
            .start()
    }

    /// STATEMENT: `GET /hello` answers with a fixed body.
    ///
    /// HOW:
    /// - Send `GET /hello` without a body.
    /// - Check the status is 200 and the body is `hello world`.
    #[ntex::test]
    async fn hello_returns_fixed_body() {
        let srv = start();
        let client = connect(srv.addr().to_string());

        let (_, recv) = client
            .send(Method::GET, "/hello".into(), HeaderMap::new(), true)
            .await
            .unwrap();
        let res = read_response(recv).await.unwrap();
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.body, "hello world");
    }

    /// STATEMENT: `POST /echo` sends back a body that arrived in several chunks.
    ///
    /// HOW:
    /// - Send `POST /echo` with the body in two data frames.
    /// - Check the status is 200 and the body is both chunks joined.
    #[ntex::test]
    async fn echo_returns_chunked_body() {
        let srv = start();
        let client = connect(srv.addr().to_string());

        let (send, recv) = client
            .send(Method::POST, "/echo".into(), HeaderMap::new(), false)
            .await
            .unwrap();
        send.send_payload("hello ", false).await.unwrap();
        send.send_payload("again", true).await.unwrap();
        let res = read_response(recv).await.unwrap();
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.body, "hello again");
    }

    /// STATEMENT: `POST /echo` answers 413 to a body over `MAX_ECHO_BODY`.
    ///
    /// HOW:
    /// - Send `POST /echo` with a body one byte over the limit, then end it.
    /// - Check the status is 413, whether or not the server's reset stopped
    ///   the upload first.
    #[ntex::test]
    async fn echo_rejects_large_body() {
        let srv = start();
        let client = connect(srv.addr().to_string());

        let (send, recv) = client
            .send(Method::POST, "/echo".into(), HeaderMap::new(), false)
            .await
            .unwrap();
        let body = Bytes::from(vec![b'x'; MAX_ECHO_BODY + 1]);
        let _ = send.send_payload(body, true).await;
        let res = read_response(recv).await.unwrap();
        assert_eq!(res.status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    /// STATEMENT: `POST /count` counts a body larger than the flow-control
    /// windows and reports the total in the body and a trailer.
    ///
    /// HOW:
    /// - Send 4 MiB to `POST /count`, more than the 1 MiB connection window,
    ///   so the upload only finishes if the counting task releases capacity.
    /// - Read the response at the same time.
    /// - Check the body and the `x-bytes` trailer carry the byte count.
    #[ntex::test]
    async fn count_reports_total_in_trailer() {
        let srv = start();
        let client = connect(srv.addr().to_string());
        let total = 4 * 1024 * 1024;

        let (send, recv) = client
            .send(Method::POST, "/count".into(), HeaderMap::new(), false)
            .await
            .unwrap();
        let (sent, res) = ntex::util::join(
            async {
                let chunk = Bytes::from(vec![b'x'; 64 * 1024]);
                for _ in 0..total / chunk.len() {
                    send.send_payload(chunk.clone(), false).await?;
                }
                send.send_payload(Bytes::new(), true).await
            },
            read_response(recv),
        )
        .await;
        sent.unwrap();
        let res = res.unwrap();
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.body, format!("counted {total} bytes"));
        assert_eq!(res.trailers.get(BYTES_TRAILER).unwrap(), &total.to_string());
    }

    /// STATEMENT: A path the server does not know answers 404.
    ///
    /// HOW:
    /// - Send `GET /missing`.
    /// - Check the status is 404 and the body is empty.
    #[ntex::test]
    async fn unknown_path_returns_not_found() {
        let srv = start();
        let client = connect(srv.addr().to_string());

        let (_, recv) = client
            .send(Method::GET, "/missing".into(), HeaderMap::new(), true)
            .await
            .unwrap();
        let res = read_response(recv).await.unwrap();
        assert_eq!(res.status, StatusCode::NOT_FOUND);
        assert!(res.body.is_empty());
    }
}
