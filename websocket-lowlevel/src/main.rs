//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser

use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::future::{select, Either};
use ntex::http::{body, h1, HttpService, Request, ResponseError};
use ntex::io::{Io, IoRef};
use ntex::service::{fn_factory, fn_service, pipeline_factory, ServiceFactory};
use ntex::web::{middleware, App};
use ntex::{channel::oneshot, rt, server, time, util::Bytes, ws};
use ntex_files as fs;
use ntex_tls::openssl::Acceptor;
use openssl::ssl::{self, SslAcceptor, SslFiletype, SslMethod};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

struct WsState {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
}

/// WebSockets service factory
async fn ws_service<F>(
    (req, io, codec): (Request, Io<F>, h1::Codec),
) -> Result<(), io::Error> {
    let state = Rc::new(RefCell::new(WsState { hb: Instant::now() }));

    match ws::handshake(req.head()) {
        // invalid websockets handshake request
        Err(e) => {
            // send http handshake respone
            io.send(
                h1::Message::Item((
                    e.error_response().drop_body(),
                    body::BodySize::None,
                )),
                &codec,
            )
            .await
            .map_err(|e| e.into_inner())?;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "WebSockets handshake error",
            ));
        }
        Ok(mut res) => {
            // send http handshake respone
            io.encode(
                h1::Message::Item((res.finish().drop_body(), body::BodySize::None)),
                &codec,
            )?;
        }
    }

    let codec = ws::Codec::new();

    // disconnect notification
    let (tx, rx) = oneshot::channel();

    // start heartbeat task
    rt::spawn(heartbeat(io.get_ref(), state.clone(), codec.clone(), rx));

    // websockets handler service
    loop {
        match io.recv(&codec).await {
            Ok(Some(frame)) => {
                println!("WS Frame: {:?}", frame);

                let item = match frame {
                    ws::Frame::Ping(msg) => {
                        (*state.borrow_mut()).hb = Instant::now();
                        ws::Message::Pong(msg)
                    }
                    ws::Frame::Text(text) => ws::Message::Text(
                        String::from_utf8(Vec::from(text.as_ref())).unwrap().into(),
                    ),
                    ws::Frame::Binary(bin) => ws::Message::Binary(bin),
                    ws::Frame::Close(reason) => ws::Message::Close(reason),
                    _ => ws::Message::Close(None),
                };
                if let Err(e) = io.send(item, &codec).await {
                    println!("Error during sending response: {:?}", e);
                    break;
                }
                continue;
            }
            Ok(None) => println!("Connection is dropped"),
            Err(err) => println!("Connection is dropped with error: {:?}", err),
        }
        break;
    }
    let _ = tx.send(());

    return Ok(());
}

/// helper method that sends ping to client every heartbeat interval
async fn heartbeat(
    io: IoRef,
    state: Rc<RefCell<WsState>>,
    codec: ws::Codec,
    mut rx: oneshot::Receiver<()>,
) {
    loop {
        match select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            Either::Left(_) => {
                // check client heartbeats
                if Instant::now().duration_since(state.borrow().hb) > CLIENT_TIMEOUT {
                    // heartbeat timed out
                    println!("Websocket Client heartbeat failed, disconnecting!");
                    io.close();
                    return;
                }

                // send ping
                if io.encode(ws::Message::Ping(Bytes::new()), &codec).is_err() {
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

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=trace");
    env_logger::init();

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("../openssl/key.pem", SslFiletype::PEM)
        .unwrap();
    builder
        .set_certificate_chain_file("../openssl/cert.pem")
        .unwrap();
    builder.set_options(ssl::SslOptions::NO_COMPRESSION);
    builder.set_mode(ssl::SslMode::RELEASE_BUFFERS);
    builder.set_read_ahead(false);
    let acceptor = builder.build();

    server::Server::build()
        // start http server on 127.0.0.1:8080
        .bind("http", "127.0.0.1:8080", move |_| {
            pipeline_factory(Acceptor::new(acceptor.clone()))
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "ssl error"))
                .and_then(
                    HttpService::build()
                        // websocket handler, we need to verify websocket handshake
                        // and then switch to websokets streaming
                        .upgrade(fn_factory(|| async {
                            Ok::<_, io::Error>(fn_service(ws_service))
                        }))
                        .finish(
                            App::new()
                                // enable logger
                                .wrap(middleware::Logger::default())
                                // static files
                                .service(
                                    fs::Files::new("/", "static/")
                                        .index_file("index.html"),
                                ),
                        )
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "http error")),
                )
        })?
        .run()
        .await
}
