//! `ChatServer` maintains list of connected client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `ChatServer`.

use futures::channel::mpsc::{self, UnboundedSender};
use futures::{SinkExt, StreamExt};
use rand::{self, rngs::ThreadRng, Rng};

use ntex::rt;
use ntex::util::{HashMap, HashSet};

/// Chat server sends this messages to session
#[derive(Debug)]
pub enum ClientMessage {
    Id(usize),
    Message(String),
    Rooms(Vec<String>),
}

/// Message for chat server communications
pub enum ServerMessage {
    /// New chat session is created
    Connect(UnboundedSender<ClientMessage>),
    /// Client session is closed
    Disconnect(usize),
    /// Send message to specific room
    Message {
        /// Id of the client session
        id: usize,
        /// Peer message
        msg: String,
        /// Room name
        room: String,
    },
    /// List of available rooms
    ListRooms(usize),
    /// Join room, if room does not exists create new one.
    Join {
        /// Client id
        id: usize,
        /// Room name
        name: String,
    },
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct ChatServer {
    sessions: HashMap<usize, UnboundedSender<ClientMessage>>,
    rooms: HashMap<String, HashSet<usize>>,
    rng: ThreadRng,
}

impl Default for ChatServer {
    fn default() -> ChatServer {
        // default room
        let mut rooms = HashMap::default();
        rooms.insert("Main".to_owned(), HashSet::default());

        ChatServer {
            rooms,
            sessions: HashMap::default(),
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    /// Send message to all users in the room
    fn send_message(&mut self, room: &str, message: &str, skip_id: usize) {
        if let Some(sessions) = self.rooms.get(room) {
            for id in sessions {
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        let msg = message.to_owned();
                        let mut addr = addr.clone();
                        rt::spawn(async move {
                            let _ = addr.send(ClientMessage::Message(msg)).await;
                        });
                    }
                }
            }
        }
    }

    /// Handler for server messages.
    fn handle(&mut self, msg: ServerMessage) {
        match msg {
            // Register new session and assign unique id to this session
            ServerMessage::Connect(mut sender) => {
                println!("Someone joined");

                // notify all users in same room
                self.send_message("Main", "Someone joined", 0);

                // register session with random id
                let id = self.rng.gen::<usize>();
                self.sessions.insert(id, sender.clone());

                // auto join session to Main room
                self.rooms.entry("Main".to_owned()).or_default().insert(id);

                // send id back
                rt::spawn(async move {
                    let _ = sender.send(ClientMessage::Id(id)).await;
                });
            }

            // Handler for Disconnect message.
            ServerMessage::Disconnect(id) => {
                println!("Someone disconnected");

                let mut rooms: Vec<String> = Vec::new();

                // remove address
                if self.sessions.remove(&id).is_some() {
                    // remove session from all rooms
                    for (name, sessions) in &mut self.rooms {
                        if sessions.remove(&id) {
                            rooms.push(name.to_owned());
                        }
                    }
                }
                // send message to other users
                for room in rooms {
                    self.send_message(&room, "Someone disconnected", 0);
                }
            }

            // Handler for Message message.
            ServerMessage::Message { id, msg, room } => {
                self.send_message(&room, msg.as_str(), id);
            }

            // Handler for `ListRooms` message.
            ServerMessage::ListRooms(id) => {
                let mut rooms = Vec::new();

                for key in self.rooms.keys() {
                    rooms.push(key.to_owned())
                }

                if let Some(addr) = self.sessions.get(&id) {
                    let mut addr = addr.clone();
                    rt::spawn(async move {
                        let _ = addr.send(ClientMessage::Rooms(rooms)).await;
                    });
                }
            }

            // Join room, send disconnect message to old room
            // send join message to new room
            ServerMessage::Join { id, name } => {
                let mut rooms = Vec::new();

                // remove session from all rooms
                for (n, sessions) in &mut self.rooms {
                    if sessions.remove(&id) {
                        rooms.push(n.to_owned());
                    }
                }
                // send message to other users
                for room in rooms {
                    self.send_message(&room, "Someone disconnected", 0);
                }

                self.rooms.entry(name.clone()).or_default().insert(id);

                self.send_message(&name, "Someone connected", id);
            }
        }
    }
}

pub fn start() -> UnboundedSender<ServerMessage> {
    let (tx, mut rx) = mpsc::unbounded();

    rt::Arbiter::new().exec_fn(move || {
        rt::spawn(async move {
            let mut srv = ChatServer::default();

            while let Some(msg) = rx.next().await {
                srv.handle(msg);
            }

            rt::Arbiter::current().stop();
        });
    });

    tx
}
