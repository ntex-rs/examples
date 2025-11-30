//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser

use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::future::{ready, select, Either};
use ntex::service::{fn_factory_with_config, fn_shutdown, Service};
use ntex::util::Bytes;
use ntex::web::{self, middleware, ws, App, Error, HttpRequest, HttpResponse};
use ntex::{chain, fn_service};
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

/// WebSockets service factory
async fn ws_service(
    sink: ws::WsSink,
) -> Result<
    impl Service<ws::Frame, Response = Option<ws::Message>, Error = io::Error>,
    web::Error,
> {
    let state = Rc::new(RefCell::new(WsState { hb: Instant::now() }));

    // disconnect notification
    let (tx, rx) = oneshot::channel();

    // start heartbeat task
    rt::spawn(heartbeat(state.clone(), sink, rx));

    // handler service for incoming websockets frames
    let service = fn_service(move |frame| {
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
            // close connection
            ws::Frame::Close(reason) => Some(ws::Message::Close(reason)),
            // ignore other frames
            _ => None,
        };
        ready(Ok(item))
    });

    // handler service for shutdown notification that stop heartbeat task
    let on_shutdown = fn_shutdown(move || {
        let _ = tx.send(());
    });

    // pipe our service with on_shutdown callback
    Ok(chain(service).and_then(on_shutdown))
}

/// helper method that sends ping to client every heartbeat interval
async fn heartbeat(
    state: Rc<RefCell<WsState>>,
    sink: ws::WsSink,
    mut rx: oneshot::Receiver<()>,
) {
    loop {
        match select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            Either::Left(_) => {
                // check client heartbeats
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

/// do websocket handshake and start web sockets service
async fn ws_index(req: HttpRequest) -> Result<HttpResponse, Error> {
    ws::start(req, fn_factory_with_config(ws_service)).await
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=trace,trace");
    env_logger::init();

    web::server(async || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            // static files
            .service(fs::Files::new("/", "static/").index_file("index.html"))
    })
    // start http server on 127.0.0.1:8080
    .bind("127.0.0.1:8080")?
    .workers(1)
    .run()
    .await
}
