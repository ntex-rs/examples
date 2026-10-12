use std::{cell::RefCell, io, rc::Rc, time::Duration, time::Instant};

use futures::channel::mpsc::{self, UnboundedSender};
use futures::{SinkExt, StreamExt, future::ready};

use ntex::service::{Service, fn_service_st, service as chain_service};
use ntex::web::{self, App, HttpRequest, HttpResponse, ws};
use ntex::{channel::oneshot, http, io::Io, rt, time, util, util::ByteString, util::Bytes};
use ntex_files as fs;

use super::server::{ClientMessage, ServerMessage};

#[derive(Clone)]
struct ChatState(mpsc::UnboundedSender<ServerMessage>);

impl web::State for ChatState {
    type Error = web::DefaultError;
}

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Upgrades an HTTP request to a WebSocket chat session.
async fn chat_route(srv: &ChatState, _: (), req: HttpRequest) -> HttpResponse {
    let srv = srv.0.clone();
    if let Ok(service) = ws_service(srv).await {
        let _ = ws::start(&req, None::<&str>, service).await;
    }
    HttpResponse::Ok().build()
}

struct WsChatSession {
    /// Unique session ID.
    id: usize,
    /// Time of the most recent heartbeat from the client.
    hb: Instant,
    /// Room currently joined by the client.
    room: String,
    /// Optional display name.
    name: Option<String>,
    /// Connection to the chat server.
    server: mpsc::UnboundedSender<ServerMessage>,
}

impl Drop for WsChatSession {
    fn drop(&mut self) {
        // Notify the chat server when this session closes.
        let _ = self.server.send(ServerMessage::Disconnect(self.id));
    }
}

/// Creates the WebSocket service for one client.
async fn ws_service(
    mut server: mpsc::UnboundedSender<ServerMessage>,
) -> Result<
    impl Service<ws::WsSink, ws::Frame, Res = Option<ws::Message>, Error = io::Error>,
    io::Error,
> {
    let (tx, mut rx) = mpsc::unbounded();

    // Register this client with the chat server.
    server.send(ServerMessage::Connect(tx)).await.unwrap();

    // The first server message contains the assigned session ID.
    let id = if let Some(ClientMessage::Id(id)) = rx.next().await {
        id
    } else {
        panic!();
    };

    // Create state for this WebSocket session.
    let state = Rc::new(RefCell::new(WsChatSession {
        id,
        hb: Instant::now(),
        server: server.clone(),
        room: "Main".to_owned(),
        name: None,
    }));

    let (tx, heartbeat_rx) = oneshot::channel();
    let tasks = Rc::new(RefCell::new(Some((rx, heartbeat_rx))));

    // Handle incoming WebSocket frames.
    let task_state = state.clone();
    let task_server = server.clone();
    let service = fn_service_st(move |_: &ws::WsSink, frame| {
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

                // Messages beginning with `/` are chat commands.
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
                                state.borrow_mut().room.clone_from(&room);
                                let mut srv = server.clone();
                                rt::spawn(async move {
                                    let _ = srv.send(ServerMessage::Join { id, name: room }).await;
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
                    // Forward the message to the chat server.
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

    // Stop the heartbeat task when the WebSocket service shuts down.
    let service = chain_service(service)
        .readiness(async move |sink| {
            if let Some((messages_rx, heartbeat_rx)) = tasks.borrow_mut().take() {
                rt::spawn(messages(sink.clone(), messages_rx));
                rt::spawn(heartbeat(
                    task_state.clone(),
                    sink.clone(),
                    task_server.clone(),
                    heartbeat_rx,
                ));
            }
            Ok::<_, io::Error>(())
        })
        .shutdown(async move |_| {
            let _ = tx.send(());
        });

    Ok(service)
}

/// Forwards chat-server messages to the WebSocket client.
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

/// Sends heartbeat pings and disconnects unresponsive clients.
async fn heartbeat(
    state: Rc<RefCell<WsChatSession>>,
    sink: ws::WsSink,
    mut server: mpsc::UnboundedSender<ServerMessage>,
    mut rx: oneshot::Receiver<()>,
) {
    loop {
        match util::select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            util::Either::Left(_) => {
                // Check whether the client has responded recently.
                if Instant::now().duration_since(state.borrow().hb) > CLIENT_TIMEOUT {
                    // heartbeat timed out
                    println!("Websocket Client heartbeat failed, disconnecting!");

                    // Notify the chat server.
                    let _ = server.send(ServerMessage::Disconnect(state.borrow().id));

                    // Disconnect the client.
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

pub fn server(
    server: UnboundedSender<ServerMessage>,
) -> impl Service<(), Io, Res = (), Error = http::error::DispatchError> {
    // Start the HTTP server with WebSocket support.
    http::HttpService::new(
        App::<ChatState, ()>::new()
            // Redirect browsers to the chat page.
            .service(
                web::resource::<ChatState, (), _>("/").route(web::get::<ChatState, ()>().to(
                    || async {
                        HttpResponse::Found()
                            .header("LOCATION", "/static/websocket.html")
                            .build()
                    },
                )),
            )
            // Accept WebSocket connections.
            .service(web::resource::<ChatState, (), _>("/ws/").to_with_state(chat_route))
            // Serve the browser client assets.
            .service(fs::Files::new("/static/", "static/"))
            .build_with(ChatState(server)),
    )
}
