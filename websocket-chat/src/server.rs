//! `ChatServer` tracks connected sessions and available rooms. Peers use it to
//! send messages to other members of the same room.

use rand;
use std::collections::{HashMap, HashSet};

use futures::channel::mpsc::{self, UnboundedSender};
use futures::{SinkExt, StreamExt};
use ntex::rt;

/// A message sent from the chat server to a client session.
#[derive(Debug)]
pub enum ClientMessage {
    Id(usize),
    Message(String),
    Rooms(Vec<String>),
}

/// A message sent to the chat server.
pub enum ServerMessage {
    /// A new client session connected.
    Connect(UnboundedSender<ClientMessage>),
    /// A client session disconnected.
    Disconnect(usize),
    /// Send a message to a room.
    Message {
        /// Client session ID.
        id: usize,
        /// Message text.
        msg: String,
        /// Room name.
        room: String,
    },
    /// List the available rooms.
    ListRooms(usize),
    /// Join a room, creating it if necessary.
    Join {
        /// Client session ID.
        id: usize,
        /// Room name.
        name: String,
    },
}

/// Coordinates client sessions and chat rooms.
pub struct ChatServer {
    sessions: HashMap<usize, UnboundedSender<ClientMessage>>,
    rooms: HashMap<String, HashSet<usize>>,
}

impl Default for ChatServer {
    fn default() -> ChatServer {
        // Create the default room.
        let mut rooms = HashMap::new();
        rooms.insert("Main".to_owned(), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
        }
    }
}

impl ChatServer {
    /// Sends a message to every client in a room.
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

    /// Handles a message sent to the server.
    fn handle(&mut self, msg: ServerMessage) {
        match msg {
            // Register the new session with a random ID.
            ServerMessage::Connect(mut sender) => {
                println!("Someone joined");

                // Notify the existing room members.
                self.send_message("Main", "Someone joined", 0);

                // Store the new session.
                let id = rand::random::<usize>();
                self.sessions.insert(id, sender.clone());

                // Join the default room.
                self.rooms
                    .entry("Main".to_owned())
                    .or_insert_with(HashSet::new)
                    .insert(id);

                // Send the assigned ID to the client.
                rt::spawn(async move {
                    let _ = sender.send(ClientMessage::Id(id)).await;
                });
            }

            // Remove a disconnected session.
            ServerMessage::Disconnect(id) => {
                println!("Someone disconnected");

                let mut rooms: Vec<String> = Vec::new();

                // Remove the session.
                if self.sessions.remove(&id).is_some() {
                    // Remove the session from every room.
                    for (name, sessions) in &mut self.rooms {
                        if sessions.remove(&id) {
                            rooms.push(name.to_owned());
                        }
                    }
                }
                // Notify the remaining room members.
                for room in rooms {
                    self.send_message(&room, "Someone disconnected", 0);
                }
            }

            // Forward a chat message to its room.
            ServerMessage::Message { id, msg, room } => {
                self.send_message(&room, msg.as_str(), id);
            }

            // Return the list of rooms.
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

            // Leave the old rooms and notify their members before joining the
            // new room.
            ServerMessage::Join { id, name } => {
                let mut rooms = Vec::new();

                // Remove the session from every room.
                for (n, sessions) in &mut self.rooms {
                    if sessions.remove(&id) {
                        rooms.push(n.to_owned());
                    }
                }
                // Notify the remaining room members.
                for room in rooms {
                    self.send_message(&room, "Someone disconnected", 0);
                }

                self.rooms
                    .entry(name.clone())
                    .or_insert_with(HashSet::new)
                    .insert(id);

                self.send_message(&name, "Someone connected", id);
            }
        }
    }
}

pub fn start() -> UnboundedSender<ServerMessage> {
    let (tx, mut rx) = mpsc::unbounded();

    rt::Arbiter::new().handle().spawn(async move {
        let mut srv = ChatServer::default();

        while let Some(msg) = rx.next().await {
            srv.handle(msg);
        }

        rt::Arbiter::current().stop();
    });

    tx
}
