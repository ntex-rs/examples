//! A gRPC server with ntex-grpc: every kind of method, several services on
//! one server, a fallback for unknown services and server configuration.
//!
//! [`echo`] is generated from `echo.proto` by `ntex-grpc-codegen` (see
//! `generate.sh`). It has two services, `echo.Echo` and `echo.Stats`. This
//! crate implements both and puts them behind one [`server::GrpcServer`].
use std::convert::Infallible;
use std::sync::{Arc, atomic::AtomicU64, atomic::Ordering};

use ntex::http::header::HeaderName;
use ntex::{SharedCfg, time::Millis, time::Seconds, time::sleep};
use ntex_grpc::server::{GrpcServer, GrpcStatus, Request, Response, ServerError};
use ntex_grpc::{
    Ctx, HeaderValue, RecvStream, SendStream, Service, ServiceFactory, encode_grpc_message, server,
};
use ntex_h2 as h2;

pub mod echo;

use self::echo::{EchoReply, EchoRequest, StatsReply, StatsRequest};

/// Address the example server listens on and the client connects to.
pub const ADDR: &str = "127.0.0.1:50055";

/// Largest request message the server accepts, in bytes.
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024;

/// Largest `repeat` of a `ServerStreamingEcho` call.
pub const MAX_REPEAT: u32 = 100;

/// Request header whose value `UnaryEcho` puts before the message.
pub const PREFIX_HEADER: &str = "x-echo-prefix";

/// Trailer of a `ClientStreamingEcho` reply with the number of messages it
/// joined.
pub const COUNT_TRAILER: HeaderName = HeaderName::from_static("x-message-count");

/// Implementation of the `echo.Echo` service.
///
/// `GrpcServer` creates an instance for every connection, so the call
/// counter is shared through an `Arc`: workers run on their own threads.
#[derive(Clone, Debug)]
pub struct EchoServer {
    calls: Arc<AtomicU64>,
}

#[server(crate::echo::Echo)]
impl EchoServer {
    /// Unary. `Request<_>` gives the headers along with the message.
    /// Returning an error ends the call with its status, an empty message
    /// is rejected with `INVALID_ARGUMENT`.
    #[method(UnaryEcho)]
    async fn unary_echo(
        &self,
        req: Request<EchoRequest>,
    ) -> Result<Response<EchoReply>, ServerError> {
        self.count();
        if req.message.message.is_empty() {
            return Err(ServerError::new(
                GrpcStatus::InvalidArgument,
                encode_grpc_message("message must not be empty"),
                None,
            ));
        }
        if req.delay_ms > 0 {
            sleep(Millis(req.delay_ms)).await;
        }
        let prefix = req
            .headers
            .get(PREFIX_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let reply = EchoReply {
            message: format!("{prefix}{}", req.message.message).into(),
        };
        Ok(reply.into())
    }

    /// Server streaming. Returning an error ends the call with its status.
    #[method(ServerStreamingEcho)]
    async fn server_streaming_echo(
        &self,
        req: EchoRequest,
        tx: SendStream<EchoReply>,
    ) -> Result<(), ServerError> {
        self.count();
        if req.repeat > MAX_REPEAT {
            return Err(ServerError::new(
                GrpcStatus::InvalidArgument,
                encode_grpc_message(&format!("repeat must be at most {MAX_REPEAT}")),
                None,
            ));
        }
        let reply = EchoReply {
            message: req.message,
        };
        for _ in 0..req.repeat {
            // `?` turns a failed send, e.g. the client went away, into
            // the status of the call
            tx.send(&reply).await?;
        }
        Ok(())
    }

    /// Client streaming. The reply is sent once the client closes its
    /// stream, with the number of messages in an `x-message-count` trailer.
    #[method(ClientStreamingEcho)]
    async fn client_streaming_echo(&self, mut rx: RecvStream<EchoRequest>) -> Response<EchoReply> {
        self.count();
        let mut messages = Vec::new();
        while let Some(Ok(req)) = rx.recv().await {
            messages.push(req.message);
        }
        Response {
            headers: vec![(COUNT_TRAILER, HeaderValue::from(messages.len()))],
            message: EchoReply {
                message: messages.join(" ").into(),
            },
        }
    }

    /// Bidirectional streaming. Every message is answered as it arrives.
    #[method(BidiStreamingEcho)]
    async fn bidi_streaming_echo(
        &self,
        mut rx: RecvStream<EchoRequest>,
        tx: SendStream<EchoReply>,
    ) -> Result<(), ServerError> {
        self.count();
        while let Some(req) = rx.recv().await {
            let req = req?;
            tx.send(&EchoReply {
                message: req.message,
            })
            .await?;
        }
        Ok(())
    }
}

impl EchoServer {
    fn count(&self) {
        self.calls.fetch_add(1, Ordering::Relaxed);
    }
}

impl ServiceFactory<(), server::ServerRequest> for EchoServer {
    type Res = server::ServerResponse;
    type Error = server::ServerError;
    type Service = EchoServer;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(self.clone())
    }
}

/// Implementation of the `echo.Stats` service.
#[derive(Clone, Debug)]
pub struct StatsServer {
    calls: Arc<AtomicU64>,
}

#[server(crate::echo::Stats)]
impl StatsServer {
    #[method(GetStats)]
    async fn get_stats(&self, _: StatsRequest) -> StatsReply {
        StatsReply {
            calls: self.calls.load(Ordering::Relaxed),
        }
    }
}

impl ServiceFactory<(), server::ServerRequest> for StatsServer {
    type Res = server::ServerResponse;
    type Error = server::ServerError;
    type Service = StatsServer;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(self.clone())
    }
}

/// Answers the calls for the services the server does not have.
///
/// A hand-written service: it gets the raw [`server::ServerRequest`], before
/// any decoding, and answers with a [`server::ServerResponse`] or a
/// [`ServerError`]. Without it, such calls fail with a plain
/// `UNIMPLEMENTED`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Fallback;

impl Service<(), server::ServerRequest> for Fallback {
    type Res = server::ServerResponse;
    type Error = ServerError;

    async fn call(
        &self,
        req: server::ServerRequest,
        _: Ctx<'_, Self, ()>,
    ) -> Result<Self::Res, Self::Error> {
        let msg = format!(
            "unknown service {}, this server has echo.Echo and echo.Stats",
            req.service
        );
        Err(ServerError::new(
            GrpcStatus::Unimplemented,
            encode_grpc_message(&msg),
            None,
        ))
    }
}

impl ServiceFactory<(), server::ServerRequest> for Fallback {
    type Res = server::ServerResponse;
    type Error = ServerError;
    type Service = Fallback;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(Fallback)
    }
}

/// The gRPC server for one connection.
///
/// Calls are routed by the service name of their path: `echo.Echo` and
/// `echo.Stats` go to their services, anything else to [`Fallback`].
pub fn grpc_server(calls: Arc<AtomicU64>) -> GrpcServer {
    GrpcServer::new(EchoServer {
        calls: calls.clone(),
    })
    .service(StatsServer { calls })
    .default_service(Fallback)
    .max_message_size(MAX_MESSAGE_SIZE)
}

/// Server configuration: HTTP/2 settings of every connection.
///
/// `GrpcServer` reads them from the configuration of the listener it is
/// bound with.
pub fn server_cfg() -> SharedCfg {
    SharedCfg::new("GRPC")
        .add(
            h2::ServiceConfig::new()
                // parallel calls per connection
                .set_max_concurrent_streams(256)
                // per-call and per-connection receive windows
                .set_initial_window_size(256 * 1024)
                .set_initial_connection_window_size(1024 * 1024)
                // close connections that stop answering pings
                .set_ping_timeout(Seconds(20))
                .set_handshake_timeout(Seconds(5)),
        )
        .into()
}

/// Builds an `Echo` client and a `Stats` client for `addr`, sharing one
/// h2 connection pool.
pub fn connect(
    addr: impl Into<String>,
) -> (
    echo::EchoClient<ntex_grpc::client::Client>,
    echo::StatsClient<ntex_grpc::client::Client>,
) {
    let pool = h2::client::Client::builder(addr.into()).build(SharedCfg::new("GRPC-CLIENT"));
    let client = ntex_grpc::client::Client::new(pool);
    (
        echo::EchoClient::new(client.clone()),
        echo::StatsClient::new(client),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{server::TestServer, server::TestServerBuilder, util::join};
    use ntex_grpc::client::{ClientError, Request as ClientRequest};
    use ntex_grpc::{ByteString, MethodDef, StreamError};

    fn echo_request(message: &str) -> EchoRequest {
        EchoRequest {
            message: message.into(),
            ..Default::default()
        }
    }

    /// Serves `grpc_server()` with the configuration of the binary.
    fn start() -> TestServer {
        let calls = Arc::new(AtomicU64::new(0));
        TestServerBuilder::new(async move |_| grpc_server(calls.clone()))
            .config(server_cfg())
            .start()
    }

    /// A method of a service the server does not have.
    struct UnknownCall;

    impl MethodDef for UnknownCall {
        const NAME: &'static str = "Call";
        const PATH: ByteString = ByteString::from_static("/echo.Unknown/Call");
        type Input = EchoRequest;
        type Output = EchoReply;
    }

    /// STATEMENT: A unary handler that takes `Request<_>` reads the request headers.
    ///
    /// HOW:
    /// - Call `UnaryEcho` with the `x-echo-prefix` header set to `> `.
    /// - Check the reply is the message with the prefix in front.
    #[ntex::test]
    async fn unary_echo_reads_request_headers() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let msg = echo_request("hello");
        let mut req = echo.unary_echo(&msg);
        req.header(PREFIX_HEADER, "> ").timeout(Seconds(3));
        assert_eq!(req.send().await.unwrap().message, "> hello");
    }

    /// STATEMENT: A `ServerError` returned by a unary handler becomes the status and message of the call.
    ///
    /// HOW:
    /// - Call `UnaryEcho` with an empty message.
    /// - Check the call fails with `INVALID_ARGUMENT` and the handler's
    ///   message.
    #[ntex::test]
    async fn unary_echo_rejects_an_empty_message() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let err = echo.unary_echo(&echo_request("")).send().await.unwrap_err();
        let ClientError::Status(status) = &*err else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(status.code, GrpcStatus::InvalidArgument);
        assert_eq!(err.message(), "message must not be empty");
    }

    /// STATEMENT: A server-streaming handler sends one reply per `tx.send()` and the call ends with `OK`.
    ///
    /// HOW:
    /// - Call `ServerStreamingEcho` with `repeat: 3`.
    /// - Collect the stream and check it has three copies of the message
    ///   and the final status is `OK`.
    #[ntex::test]
    async fn server_streaming_repeats_the_message() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let msg = EchoRequest {
            repeat: 3,
            ..echo_request("again")
        };
        let mut rx = echo.server_streaming_echo(&msg).open().await.unwrap();
        let mut replies = Vec::new();
        while let Some(reply) = rx.recv().await {
            replies.push(reply.unwrap().message);
        }
        assert_eq!(replies, ["again", "again", "again"]);
        assert_eq!(rx.status(), Some(GrpcStatus::Ok));
    }

    /// STATEMENT: A `ServerError` returned by a streaming handler becomes the status and message of the call.
    ///
    /// HOW:
    /// - Call `ServerStreamingEcho` with `repeat` above `MAX_REPEAT`.
    /// - Check the stream ends with `INVALID_ARGUMENT` and the handler's
    ///   message, without any reply.
    #[ntex::test]
    async fn server_streaming_rejects_a_large_repeat() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let msg = EchoRequest {
            repeat: MAX_REPEAT + 1,
            ..echo_request("x")
        };
        let mut rx = echo.server_streaming_echo(&msg).open().await.unwrap();
        let err = rx.recv().await.unwrap().unwrap_err();
        assert_eq!(err.status(), GrpcStatus::InvalidArgument);
        let StreamError::Client(err) = err else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(err.message(), "repeat must be at most 100");
        assert!(rx.recv().await.is_none());
    }

    /// STATEMENT: A client-streaming handler replies once the client closes its stream, with the trailers of its `Response`.
    ///
    /// HOW:
    /// - Send three messages on the request stream, then close it, while
    ///   `send()` waits for the reply.
    /// - Check the reply joins the three messages in order and the
    ///   `x-message-count` trailer is `3`.
    #[ntex::test]
    async fn client_streaming_joins_the_messages() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let tx = SendStream::new();
        let (res, sent) = join(echo.client_streaming_echo(&tx).send(), async {
            for word in ["one", "two", "three"] {
                tx.send(&echo_request(word)).await?;
            }
            tx.close();
            Ok::<_, StreamError>(())
        })
        .await;
        sent.unwrap();
        let res = res.unwrap();
        assert_eq!(res.message, "one two three");
        assert_eq!(res.trailers().get(COUNT_TRAILER).unwrap(), "3");
    }

    /// STATEMENT: A bidirectional handler answers each message before the client closes its stream.
    ///
    /// HOW:
    /// - Open `BidiStreamingEcho` and, for each word, send it and wait for
    ///   its reply before sending the next one.
    /// - Close the request stream and check the response stream ends with
    ///   `OK`.
    #[ntex::test]
    async fn bidi_streaming_answers_each_message() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let (tx, mut rx) = echo.bidi_streaming_echo().open().await.unwrap();
        for word in ["ping", "pong"] {
            tx.send(&echo_request(word)).await.unwrap();
            assert_eq!(rx.recv().await.unwrap().unwrap().message, word);
        }
        tx.close();
        assert!(rx.recv().await.is_none());
        assert_eq!(rx.status(), Some(GrpcStatus::Ok));
    }

    /// STATEMENT: Two services on one server share state across connections.
    ///
    /// HOW:
    /// - Make two `UnaryEcho` calls, each from its own client and so its own
    ///   connection.
    /// - Ask `echo.Stats` through a third client and check it counted both
    ///   calls.
    #[ntex::test]
    async fn stats_counts_calls_from_every_connection() {
        let srv = start();
        for _ in 0..2 {
            let (echo, _) = connect(srv.addr().to_string());
            echo.unary_echo(&echo_request("hi")).send().await.unwrap();
        }
        let (_, stats) = connect(srv.addr().to_string());
        let res = stats.get_stats(&StatsRequest {}).send().await.unwrap();
        assert_eq!(res.calls, 2);
    }

    /// STATEMENT: A call that outlives its deadline fails with `DEADLINE_EXCEEDED`.
    ///
    /// HOW:
    /// - Call `UnaryEcho` with `delay_ms: 500` and a 50 ms client timeout,
    ///   which is also sent to the server as `grpc-timeout`.
    /// - Check the call fails with `DEADLINE_EXCEEDED`.
    #[ntex::test]
    async fn slow_call_exceeds_its_deadline() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let msg = EchoRequest {
            delay_ms: 500,
            ..echo_request("slow")
        };
        let mut req = echo.unary_echo(&msg);
        req.timeout(Millis(50));
        let err = req.send().await.unwrap_err();
        assert_eq!(err.status(), GrpcStatus::DeadlineExceeded);
    }

    /// STATEMENT: A request larger than `max_message_size` fails with `RESOURCE_EXHAUSTED`.
    ///
    /// HOW:
    /// - Call `UnaryEcho` with a message one byte over `MAX_MESSAGE_SIZE`.
    /// - Check the server rejects it with `RESOURCE_EXHAUSTED`.
    #[ntex::test]
    async fn oversized_request_is_rejected() {
        let srv = start();
        let (echo, _) = connect(srv.addr().to_string());
        let msg = echo_request(&"a".repeat(MAX_MESSAGE_SIZE + 1));
        let err = echo.unary_echo(&msg).send().await.unwrap_err();
        assert!(matches!(&*err, ClientError::Status(_)), "{err:?}");
        assert_eq!(err.status(), GrpcStatus::ResourceExhausted);
    }

    /// STATEMENT: A call to a service the server does not have goes to the default service.
    ///
    /// HOW:
    /// - Send a unary call to `/echo.Unknown/Call` with a hand-written
    ///   `MethodDef` and the same transport as the generated clients.
    /// - Check it fails with `UNIMPLEMENTED` and the message of `Fallback`.
    #[ntex::test]
    async fn unknown_service_goes_to_the_fallback() {
        let srv = start();
        let pool = h2::client::Client::builder(srv.addr().to_string())
            .build(SharedCfg::new("GRPC-CLIENT"));
        let client = ntex_grpc::client::Client::new(pool);
        let msg = echo_request("hi");
        let err = ClientRequest::<_, UnknownCall>::new(&client, &msg)
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), GrpcStatus::Unimplemented);
        assert_eq!(
            err.message(),
            "unknown service echo.Unknown, this server has echo.Echo and echo.Stats"
        );
    }
}
