//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser

use std::cell::Cell;
use std::pin::Pin;
use std::task::Poll;
use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::Future;
use futures::future::{ready, select, Either};
use ntex::service::{fn_factory_with_config, Service};
use ntex::util::Bytes;
use ntex::web::{self, middleware, ws, App, Error, HttpRequest, HttpResponse};
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

#[inline]
fn fn_shutdown() {}

pub struct WsService<F, Fut, FShut = fn()>
where
    F: Fn(ws::Frame) -> Fut,
    Fut: Future<Output = Result<Option<ws::Message>, io::Error>>,
{
    f_frame: F,
    f_shutdown: Cell<Option<FShut>>,
}

impl<F, Fut> WsService<F, Fut>
where
    F: Fn(ws::Frame) -> Fut,
    Fut: Future<Output = Result<Option<ws::Message>, io::Error>>,
{
    pub fn new(f_frame: F) -> Self {
        Self {
            f_frame,
            f_shutdown: Cell::new(Some(fn_shutdown)),
        }
    }

    /// Set function that get called on poll_shutdown method of Service trait.
    pub fn on_disconnect<FShut>(self, f: FShut) -> WsService<F, Fut, FShut>
    where
    FShut: FnOnce(),
    {
        WsService {
            f_frame: self.f_frame,
            f_shutdown: Cell::new(Some(f)),
        }
    }
}

impl<F, Fut, FShut> Service<ws::Frame> for WsService<F, Fut, FShut>
where
F: Fn(ws::Frame) -> Fut,
Fut: Future<Output = Result<Option<ws::Message>, io::Error>>,
FShut: FnOnce(), {
    type Response = Option<ws::Message>;
    type Error = io::Error;
    type Future<'f> = Pin<Box<Fut>> where Self: 'f;

    #[inline]
    fn call(&self, req: ws::Frame) -> Self::Future<'_> {
        Box::pin((self.f_frame)(req))
    }

    #[inline]
    fn poll_shutdown(&self, _: &mut std::task::Context<'_>) -> Poll<()> {
        if let Some(f) = self.f_shutdown.take() {
            f();
        }
        Poll::Ready(())
    }
}

#[inline]
/// Create `WsService` for function that can act as a `Service`
pub fn fn_ws_service<F, Fut>(
    f: F,
) -> WsService<F, Fut>
where
F: Fn(ws::Frame) -> Fut,
Fut: Future<Output = Result<Option<ws::Message>, io::Error>>,
{
    WsService::new(f)
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

    Ok(fn_ws_service(move | frame | {
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
    }).on_disconnect(move | | {
        // stop heartbeat when connection is closed
        let _ = tx.send(());
    }))
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
                if sink.send(ws::Message::Ping(
                 Bytes::default()
                )).await.is_err() {
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
