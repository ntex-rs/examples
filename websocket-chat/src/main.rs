use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::{channel::mpsc, future::ready, SinkExt, StreamExt};
use ntex::pipeline;
use ntex::service::{fn_factory_with_config, fn_service, map_config, Service, fn_shutdown};
use ntex::web::{self, ws, App, Error, HttpRequest, HttpResponse};
use ntex::{channel::oneshot, rt, time, util, util::ByteString, util::Bytes};
use ntex_files as fs;

mod server;
use self::server::{ClientMessage, ServerMessage};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our route
async fn chat_route(
    req: HttpRequest,
    srv: web::types::State<mpsc::UnboundedSender<ServerMessage>>,
) -> Result<HttpResponse, Error> {
    let srv = srv.get_ref().clone();
    ws::start(
        req,
        // inject chat server send to a ws_service factory
        map_config(fn_factory_with_config(ws_service), move |cfg| {
            (cfg, srv.clone())
        }),
    )
    .await
}

struct WsChatSession {
    /// unique session id
    id: usize,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    /// joined room
    room: String,
    /// peer name
    name: Option<String>,
    /// server connectino
    server: mpsc::UnboundedSender<ServerMessage>,
}

impl Drop for WsChatSession {
    fn drop(&mut self) {
        // notify chat server
        let _ = self.server.send(ServerMessage::Disconnect(self.id));
    }
}

/// WebSockets service factory
async fn ws_service(
    (sink, mut server): (ws::WsSink, mpsc::UnboundedSender<ServerMessage>),
) -> Result<
    impl Service<ws::Frame, Response = Option<ws::Message>, Error = io::Error>,
    web::Error,
> {
    let (tx, mut rx) = mpsc::unbounded();

    // register self in chat server.
    server.send(ServerMessage::Connect(tx)).await.unwrap();

    // read first message from server, it shoould contain session id
    let id = if let Some(ClientMessage::Id(id)) = rx.next().await {
        id
    } else {
        panic!();
    };

    // create chat session
    let state = Rc::new(RefCell::new(WsChatSession {
        id,
        hb: Instant::now(),
        server: server.clone(),
        room: "Main".to_owned(),
        name: None,
    }));

    // start server messages handler, it reads chat messages and sends to the peer
    rt::spawn(messages(sink.clone(), rx));

    // start heartbeat task
    let (tx, rx) = oneshot::channel();
    rt::spawn(heartbeat(state.clone(), sink.clone(), server.clone(), rx));

    // handler service for incoming websockets frames
    let service = fn_service(move | frame | {
            println!("WEBSOCKET MESSAGE: {:?}", frame);
    
            let item = match frame {
                ws::Frame::Ping(msg) => {
                    state.borrow_mut().hb = Instant::now();
                    Some(ws::Message::Pong(msg))
                }
                // update heartbeat
                ws::Frame::Pong(_) => {
                    state.borrow_mut().hb = Instant::now();
                    None
                }
                ws::Frame::Text(text) => {
                    let m = String::from_utf8(Vec::from(&text[..])).unwrap();
    
                    // we check for `/sss` type of messages
                    if m.starts_with('/') {
                        let v: Vec<&str> = m.splitn(2, ' ').collect();
                        match v[0] {
                            "/list" => {
                                // Send ListRooms message to chat server and wait for
                                // response
                                println!("List rooms");
                                let mut srv = server.clone();
                                rt::spawn(async move {
                                    let _ = srv.send(ServerMessage::ListRooms(id)).await;
                                });
                                None
                            }
                            "/join" => {
                                if v.len() == 2 {
                                    let room = v[1].to_owned();
                                    state.borrow_mut().room = room.clone();
                                    let mut srv = server.clone();
                                    rt::spawn(async move {
                                        let _ = srv
                                            .send(ServerMessage::Join { id, name: room })
                                            .await;
                                    });
                                    None
                                } else {
                                    Some(ws::Message::Text(ByteString::from_static(
                                        "!!! room name is required",
                                    )))
                                }
                            }
                            "/name" => {
                                if v.len() == 2 {
                                    state.borrow_mut().name = Some(v[1].to_owned());
                                    None
                                } else {
                                    Some(ws::Message::Text(ByteString::from_static(
                                        "!!! name is required",
                                    )))
                                }
                            }
                            _ => Some(ws::Message::Text(
                                format!("!!! unknown command: {:?}", m).into(),
                            )),
                        }
                    } else {
                        let msg = if let Some(ref name) = state.borrow().name {
                            format!("{}: {}", name, m)
                        } else {
                            m
                        };
                        // send message to chat server
                        let mut srv = server.clone();
                        let msg = ServerMessage::Message {
                            id,
                            msg,
                            room: state.borrow().room.clone(),
                        };
                        rt::spawn(async move { srv.send(msg).await });
                        None
                    }
                }
                ws::Frame::Binary(_) => None,
                ws::Frame::Close(reason) => Some(ws::Message::Close(reason)),
                _ => Some(ws::Message::Close(None)),
            };
            ready(Ok(item))
    });

    // handler service for shutdown notification that stop heartbeat task
    let on_shutdown = fn_shutdown(move || {
        let _ = tx.send(());
    });

    // pipe our service with on_shutdown callback
    Ok(pipeline(service).and_then(on_shutdown))
}

/// Handle messages from chat server, we simply send it to the peer websocket connection
async fn messages(sink: ws::WsSink, mut server: mpsc::UnboundedReceiver<ClientMessage>) {
    while let Some(msg) = server.next().await {
        println!("GOT chat server message: {:?}", msg);
        match msg {
            ClientMessage::Id(_) => (),
            ClientMessage::Message(text) => {
                let _ = sink.send(ws::Message::Text(text.into())).await;
            }
            ClientMessage::Rooms(rooms) => {
                for room in rooms {
                    let _ = sink.send(ws::Message::Text(room.into())).await;
                }
            }
        }
    }
}

/// helper method that sends ping to client every second.
///
/// also this method checks heartbeats from client
async fn heartbeat(
    state: Rc<RefCell<WsChatSession>>,
    sink: ws::WsSink,
    mut server: mpsc::UnboundedSender<ServerMessage>,
    mut rx: oneshot::Receiver<()>,
) {
    loop {
        match util::select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            util::Either::Left(_) => {
                // check client heartbeats
                if Instant::now().duration_since(state.borrow().hb) > CLIENT_TIMEOUT {
                    // heartbeat timed out
                    println!("Websocket Client heartbeat failed, disconnecting!");

                    // notify chat server
                    let _ = server.send(ServerMessage::Disconnect(state.borrow().id));

                    // disconnect connection
                    sink.io().close();
                    return;
                } else {
                    // send ping
                    if sink.send(ws::Message::Ping(Bytes::new())).await.is_err() {
                        return;
                    }
                }
            }
            util::Either::Right(_) => {
                println!("Connection is dropped, stop heartbeat task");
                return;
            }
        }
    }
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    // Start chat server actor
    let server = server::start();

    // Create Http server with websocket support
    web::server(move || {
        App::new()
            .state(server.clone())
            // redirect to websocket.html
            .service(web::resource("/").route(web::get().to(|| async {
                HttpResponse::Found()
                    .header("LOCATION", "/static/websocket.html")
                    .finish()
            })))
            // websocket
            .service(web::resource("/ws/").to(chat_route))
            // static resources
            .service(fs::Files::new("/static/", "static/"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
