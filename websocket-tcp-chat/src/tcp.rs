//! chat tcp server
use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

use futures::channel::mpsc::UnboundedSender;
use futures::{channel::mpsc, SinkExt, StreamExt};
use ntex::service::{fn_service, ServiceFactory};
use ntex::{channel::oneshot, io::Io, io::IoRef, rt, time, util};

use crate::codec::{ChatCodec, ChatRequest, ChatResponse};
use crate::server::{ClientMessage, ServerMessage};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// `ChatSession` actor is responsible for tcp peer communications.
struct ChatSession {
    /// unique session id
    id: usize,
    /// Client must send ping at least once per 10 seconds, otherwise we drop
    /// connection.
    hb: Instant,
    /// joined room
    room: String,
    /// peer name
    name: Option<String>,
    /// server connectino
    server: mpsc::UnboundedSender<ServerMessage>,
}

impl Drop for ChatSession {
    fn drop(&mut self) {
        // notify chat server
        let _ = self.server.send(ServerMessage::Disconnect(self.id));
    }
}

/// Handle messages from chat server, we simply send it to the peer tcp connection
async fn messages(sink: IoRef, mut server: mpsc::UnboundedReceiver<ClientMessage>) {
    while let Some(msg) = server.next().await {
        println!("GOT chat server message: {:?}", msg);
        match msg {
            ClientMessage::Id(_) => (),
            ClientMessage::Message(text) => {
                sink.encode(ChatResponse::Message(text), &ChatCodec)
                    .unwrap();
            }
            ClientMessage::Rooms(rooms) => {
                sink.encode(ChatResponse::Rooms(rooms), &ChatCodec).unwrap();
            }
        }
    }
}

/// helper method that sends ping to client every second.
///
/// also this method checks heartbeats from client
async fn heartbeat(
    state: Rc<RefCell<ChatSession>>,
    sink: IoRef,
    mut rx: oneshot::Receiver<()>,
) {
    loop {
        match util::select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await {
            util::Either::Left(_) => {
                // check client heartbeats
                if Instant::now().duration_since(state.borrow().hb) > CLIENT_TIMEOUT {
                    // heartbeat timed out
                    println!("Tcp Client heartbeat failed, disconnecting!");

                    // close connection
                    let _ = sink.close();
                    return;
                } else {
                    // send ping
                    let _ = sink.encode(ChatResponse::Ping, &ChatCodec);
                }
            }
            util::Either::Right(_) => {
                println!("Connection is dropped, stop heartbeat task");
                return;
            }
        }
    }
}

/// Start tcp server that will accept incoming tcp connection
pub fn server(
    server: UnboundedSender<ServerMessage>,
) -> impl ServiceFactory<Io, Response = (), Error = (), InitError = ()> {
    fn_service(move |io: Io| {
        let mut server = server.clone();
        async move {
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
            let state = Rc::new(RefCell::new(ChatSession {
                id,
                hb: Instant::now(),
                server: server.clone(),
                room: "Main".to_owned(),
                name: None,
            }));

            // start server messages handler, it reads chat messages and sends to the peer
            rt::spawn(messages(io.get_ref(), rx));

            // start heartbeat task
            let (tx, rx) = oneshot::channel();
            rt::spawn(heartbeat(state.clone(), io.get_ref(), rx));

            loop {
                match io.recv(&ChatCodec).await {
                    Ok(Some(msg)) => {
                        match msg {
                            ChatRequest::List => {
                                // Send ListRooms message to chat server and wait for
                                // response
                                println!("List rooms");
                                let mut srv = server.clone();
                                rt::spawn(async move {
                                    let _ = srv.send(ServerMessage::ListRooms(id)).await;
                                });
                            }
                            ChatRequest::Join(room) => {
                                state.borrow_mut().room = room.clone();
                                let mut srv = server.clone();
                                rt::spawn(async move {
                                    let _ = srv
                                        .send(ServerMessage::Join { id, name: room })
                                        .await;
                                });
                            }
                            ChatRequest::Name(name) => {
                                state.borrow_mut().name = Some(name);
                            }
                            ChatRequest::Message(msg) => {
                                // send message to chat server
                                let mut srv = server.clone();
                                let msg = ServerMessage::Message {
                                    id,
                                    msg,
                                    room: state.borrow().room.clone(),
                                };
                                rt::spawn(async move { srv.send(msg).await });
                            }
                            ChatRequest::Ping => {
                                state.borrow_mut().hb = Instant::now();
                                let _ = io.encode(ChatResponse::Ping, &ChatCodec);
                            }
                        }
                    }
                    Ok(None) => {
                        println!("Peer is gone, stop");
                        break;
                    }
                    Err(e) => {
                        println!("Error during socket read: {:?}", e);
                        break;
                    }
                }
            }
            // stop heartbeat task
            let _ = tx.send(());

            Ok(())
        }
    })
}
