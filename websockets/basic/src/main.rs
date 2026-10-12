//! Simple WebSocket echo server.
//! Open `http://localhost:8080/ws/index.html` in browser

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
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
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

#[ntex::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=trace,trace");
    }
    env_logger::init();

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            // Accept WebSocket connections.
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            // static files
            .service(fs::Files::new("/", "static/").index_file("index.html"))
    })
    // Start the HTTP server on 127.0.0.1:8080.
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("WEBSOCKET"))?
    .workers(1)
    .run()
    .await
}
