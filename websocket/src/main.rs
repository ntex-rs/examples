//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser

use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::{future::ready, SinkExt};
use ntex::service::{fn_factory_with_config, fn_service, Service};
use ntex::web::{self, middleware, ws, App, Error, HttpRequest, HttpResponse};
use ntex::{rt, util::Bytes};
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
    sink: ws::WebSocketsSink,
) -> Result<
    impl Service<Request = ws::Frame, Response = Option<ws::Message>, Error = io::Error>,
    web::Error,
> {
    let state = Rc::new(RefCell::new(WsState { hb: Instant::now() }));

    // start heartbeat task
    rt::spawn(heartbeat(state.clone(), sink.clone()));

    // websockets handler service
    Ok(fn_service(move |frame| {
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
        ready(Ok(Some(item)))
    }))
}

/// helper method that sends ping to client every heartbeat interval
async fn heartbeat(state: Rc<RefCell<WsState>>, mut sink: ws::WebSocketsSink) {
    loop {
        rt::time::delay_for(HEARTBEAT_INTERVAL).await;

        // check client heartbeats
        if Instant::now().duration_since(state.borrow().hb) > CLIENT_TIMEOUT {
            // heartbeat timed out
            println!("Websocket Client heartbeat failed, disconnecting!");
            return;
        }

        // send ping
        if sink
            .send(Ok(ws::Message::Ping(Bytes::new())))
            .await
            .is_err()
        {
            return;
        }
    }
}

/// do websocket handshake and start web sockets service
async fn ws_index(
    req: HttpRequest,
    pl: web::types::Payload,
) -> Result<HttpResponse, Error> {
    ws::start(req, pl, fn_factory_with_config(ws_service)).await
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=trace");
    env_logger::init();

    web::server(|| {
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
    .run()
    .await
}
