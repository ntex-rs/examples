//! Simple WebSocket echo server.
//! Open `http://localhost:8080/index.html` in browser

use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::future::{Either, ready, select};
use ntex::service::{fn_service_st, service as chain_service};
use ntex::util::Bytes;
use ntex::web::{self, App, HttpRequest, middleware, ws};
use ntex::{channel::oneshot, rt, time};
use ntex_files as fs;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

struct WsState {
    /// Client must send ping or pong at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
}

/// Upgrades an HTTP request and starts the WebSocket service.
async fn ws_index(req: HttpRequest) {
    let state = Rc::new(RefCell::new(WsState { hb: Instant::now() }));

    // disconnect notification
    let (tx, rx) = oneshot::channel();
    let heartbeat_rx = Rc::new(RefCell::new(Some(rx)));

    // Handle incoming WebSocket frames.
    let heartbeat_state = state.clone();
    let service = fn_service_st(move |_: &ws::WsSink, frame| {
        let item = match frame {
            // update heartbeat
            ws::Frame::Ping(msg) => {
                state.borrow_mut().hb = Instant::now();
                Some(ws::Message::Pong(msg))
            }
            // update heartbeat
            ws::Frame::Pong(_) => {
                state.borrow_mut().hb = Instant::now();
                None
            }
            // send message back
            ws::Frame::Text(text) => Some(ws::Message::Text(
                String::from_utf8(Vec::from(text.as_ref())).unwrap().into(),
            )),
            ws::Frame::Binary(bin) => Some(ws::Message::Binary(bin)),
            // Close the connection.
            ws::Frame::Close(reason) => Some(ws::Message::Close(reason)),
            // ignore other frames
            _ => None,
        };
        ready(Ok::<_, io::Error>(item))
    });

    // Stop the heartbeat task when the WebSocket service shuts down.
    let service = chain_service(service)
        .readiness(async move |sink| {
            if let Some(rx) = heartbeat_rx.borrow_mut().take() {
                rt::spawn(heartbeat(heartbeat_state.clone(), sink.clone(), rx));
            }
            Ok::<_, io::Error>(())
        })
        .shutdown(async move |_| {
            let _ = tx.send(());
        });

    let _ = ws::start(&req, None::<&str>, service).await;
}

/// Sends heartbeat pings and disconnects unresponsive clients.
async fn heartbeat(state: Rc<RefCell<WsState>>, sink: ws::WsSink, mut rx: oneshot::Receiver<()>) {
    loop {
        match select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            Either::Left(_) => {
                // Check whether the client has responded recently.
                if Instant::now().duration_since(state.borrow().hb) > CLIENT_TIMEOUT {
                    // heartbeat timed out
                    println!("Websocket Client heartbeat failed, disconnecting!");
                    sink.io().close();
                    return;
                }

                // send ping
                if sink
                    .send(ws::Message::Ping(Bytes::default()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Either::Right(_) => {
                println!("Connection is dropped, stop heartbeat task");
                return;
            }
        }
    }
}

/// Registers the WebSocket endpoint and browser assets.
fn config(cfg: &mut web::ServiceConfig) {
    cfg // Accept WebSocket connections.
        .service(web::resource("/ws/").route(web::get().to(ws_index)))
        // static files
        .service(
            fs::Files::new("/", concat!(env!("CARGO_MANIFEST_DIR"), "/static"))
                .index_file("index.html"),
        );
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=trace,trace");
    }
    env_logger::init();

    let server = web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(config)
    });
    let server = if std::env::args().any(|arg| arg == "--tls") {
        #[cfg(feature = "tls")]
        {
            use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

            let mut config = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
            config.set_certificate_chain_file("../../tls/certs/server.pem")?;
            config.set_private_key_file("../../tls/certs/server.key", SslFiletype::PEM)?;
            server.bind_openssl(
                "127.0.0.1:8443",
                config,
                ntex::SharedCfg::new("WEBSOCKET-TLS"),
            )?
        }
        #[cfg(not(feature = "tls"))]
        {
            return Err(io::Error::other("--tls requires --features tls"));
        }
    } else {
        server.bind("127.0.0.1:8080", ntex::SharedCfg::new("WEBSOCKET"))?
    };
    server.workers(1).run().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{
        http::StatusCode,
        time::{Seconds, timeout},
        web::test,
    };

    /// STATEMENT: The WebSocket endpoint echoes text and answers pings.
    ///
    /// HOW:
    /// - Start the real routes on an ephemeral test server and connect to `/ws/`.
    /// - Send text and a ping, checking the echoed text and pong within a timeout.
    #[ntex::test]
    async fn websocket_echoes_text_and_answers_ping() {
        let srv = test::server(async |_| App::new().configure(config));
        timeout(Seconds(3), async {
            let (io, codec, _) = srv.ws_at("/ws/").await.unwrap().into_inner();
            io.send(ws::Message::Text("hello".into()), &codec)
                .await
                .unwrap();
            assert_eq!(
                io.recv(&codec).await.unwrap().unwrap(),
                ws::Frame::Text(Bytes::from_static(b"hello"))
            );
            io.send(ws::Message::Ping(Bytes::from_static(b"ping")), &codec)
                .await
                .unwrap();
            assert_eq!(
                io.recv(&codec).await.unwrap().unwrap(),
                ws::Frame::Pong(Bytes::from_static(b"ping"))
            );
        })
        .await
        .unwrap();
    }

    /// STATEMENT: The WebSocket endpoint accepts only `GET` requests.
    ///
    /// HOW:
    /// - Start the real routes and send `POST /ws/`.
    /// - Check that routing responds with `405 Method Not Allowed`.
    #[ntex::test]
    async fn websocket_rejects_non_get_method() {
        let srv = test::server(async |_| App::new().configure(config));
        let response = timeout(Seconds(3), srv.post("/ws/").send())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
