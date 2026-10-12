//! Simple WebSocket echo server.
//! Open `http://localhost:8080/ws/index.html` in browser

use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::future::{Either, select};
use ntex::http::{HttpService, Request, ResponseError, body, h1};
use ntex::io::{Io, IoRef};
use ntex::service::{Pipeline, Service, fn_service, service};
use ntex::web::{App, middleware};
use ntex::{SharedCfg, channel::oneshot, rt, server, time, util::Bytes, ws};
use ntex_files as fs;
use ntex_tls::openssl::SslAcceptor;
use openssl::ssl::{self, SslFiletype, SslMethod};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

struct WsState {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
}

/// Creates the WebSocket service for one client.
async fn ws_service<F>((req, io, codec): (Request, Io<F>, h1::Codec)) -> Result<(), io::Error> {
    let state = Rc::new(RefCell::new(WsState { hb: Instant::now() }));

    match ws::handshake(req.head()) {
        // Reject an invalid WebSocket handshake.
        Err(e) => {
            // send http handshake respone
            io.send(
                h1::Message::Item((e.error_response().drop_body(), body::BodySize::None)),
                &codec,
            )
            .await
            .map_err(|_| io::Error::other("WebSockets io error"))?;

            return Err(io::Error::other("WebSockets handshake error"));
        }
        Ok(mut res) => {
            // send http handshake respone
            io.encode(
                h1::Message::Item((res.build().drop_body(), body::BodySize::None)),
                &codec,
            )
            .map_err(|_| io::Error::other("WebSockets io error"))?;
        }
    }

    let codec = ws::Codec::new();

    // disconnect notification
    let (tx, rx) = oneshot::channel();

    // start heartbeat task
    rt::spawn(heartbeat(io.get_ref(), state.clone(), codec.clone(), rx));

    // Handle incoming WebSocket frames.
    loop {
        match io.recv(&codec).await {
            Ok(Some(frame)) => {
                println!("WS Frame: {:?}", frame);

                let item = match frame {
                    ws::Frame::Ping(msg) => {
                        state.borrow_mut().hb = Instant::now();
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

    Ok(())
}

/// Sends heartbeat pings and disconnects unresponsive clients.
async fn heartbeat(
    io: IoRef,
    state: Rc<RefCell<WsState>>,
    codec: ws::Codec,
    mut rx: oneshot::Receiver<()>,
) {
    loop {
        match select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            Either::Left(_) => {
                // Check whether the client has responded recently.
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
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=trace");
    }
    env_logger::init();

    let mut builder = ssl::SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
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

    server::Server::builder()
        // Start the HTTP server on 127.0.0.1:8080.
        .bind(
            "http",
            "127.0.0.1:8080",
            SharedCfg::new("HTTP"),
            async move |_| {
                service(SslAcceptor::new(acceptor.clone()))
                    .map_err(|_| io::Error::other("ssl error"))
                    .and_then({
                        let ws_service = Rc::new(Pipeline::new((), fn_service(ws_service)));

                        HttpService::new(
                            App::new()
                                // enable logger
                                .middleware(middleware::Logger::default())
                                // static files
                                .service(fs::Files::new("/", "static/").index_file("index.html")),
                        )
                        // Verify the WebSocket handshake before starting the service.
                        // and then switch to websokets streaming
                        .h1_control(move |req: h1::Control<_, _>| {
                            let ws_service = ws_service.clone();
                            async move {
                                let ack = if let h1::Control::Upgrade(upg) = req {
                                    let (ack, io, req, codec) = upg.handle();
                                    ws_service.call((req, io, codec)).await?;
                                    ack
                                } else {
                                    req.ack()
                                };
                                Ok::<_, io::Error>(ack)
                            }
                        })
                        .map_err(|_| io::Error::other("http error"))
                    })
            },
        )?
        .run()
        .await
}
