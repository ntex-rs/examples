//! Simple websocket client.
use std::{thread, time::Duration};

use futures::{channel::mpsc, SinkExt, StreamExt};
use ntex::{channel::oneshot, rt, time, util, SharedCfg};

mod codec;
use self::codec::{ChatRequest, ChatResponse, ClientChatCodec};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[ntex::main]
async fn main() -> Result<(), std::io::Error> {
    std::env::set_var("RUST_LOG", "ntex=trace,ntex_io=info,ntex_tokio=info");
    env_logger::init();

    // open tcp connection
    let io = rt::tcp_connect("127.0.0.1:12345".parse().unwrap(), SharedCfg::default())
        .await
        .unwrap();

    println!("Tcp connection is established: {:?}", io);

    let (mut tx, mut rx) = mpsc::unbounded();

    // start console read loop
    thread::spawn(move || loop {
        let mut cmd = String::new();
        if std::io::stdin().read_line(&mut cmd).is_err() {
            println!("error");
            return;
        }

        // send text to server
        if futures::executor::block_on(tx.send(cmd)).is_err() {
            return;
        }
    });

    // read console commands
    let ioref = io.get_ref();
    rt::spawn(async move {
        while let Some(msg) = rx.next().await {
            if msg.starts_with('/') {
                let v: Vec<&str> = msg.splitn(2, ' ').collect();
                match v[0] {
                    "/list" => {
                        // Send ListRooms message to chat server and wait for
                        // response
                        println!("List rooms");
                        ioref.encode(ChatRequest::List, &ClientChatCodec).unwrap();
                    }
                    "/join" => {
                        if v.len() == 2 {
                            let room = v[1].to_owned();
                            ioref
                                .encode(ChatRequest::Join(room), &ClientChatCodec)
                                .unwrap();
                        } else {
                            println!("!!! room name is required")
                        }
                    }
                    "/name" => {
                        if v.len() == 2 {
                            ioref
                                .encode(
                                    ChatRequest::Name(v[1].to_owned()),
                                    &ClientChatCodec,
                                )
                                .unwrap();
                        } else {
                            println!("!!! name is required")
                        }
                    }
                    _ => println!("!!! unknown command: {:?}", msg),
                }
            } else {
                // send message to chat server
                ioref
                    .encode(ChatRequest::Message(msg), &ClientChatCodec)
                    .unwrap();
            }
        }
    });

    // start heartbeat task
    let ioref = io.get_ref();
    let (tx, mut rx) = oneshot::channel();
    rt::spawn(async move {
        loop {
            match util::select(Box::pin(time::sleep(HEARTBEAT_INTERVAL)), &mut rx).await
            {
                util::Either::Left(_) => {
                    // heartbeat
                    let _ = ioref.encode(ChatRequest::Ping, &ClientChatCodec);
                }
                util::Either::Right(_) => {
                    println!("Connection is dropped, stop heartbeat task");
                    return;
                }
            }
        }
    });

    // input dispatcher
    loop {
        match io.recv(&ClientChatCodec).await {
            Ok(Some(msg)) => match msg {
                ChatResponse::Ping => {}
                ChatResponse::Rooms(rooms) => println!("Available rooms: {:?}", rooms),
                ChatResponse::Joined(name) => println!("You joined {} room", name),
                ChatResponse::Message(msg) => println!("{}", msg),
            },
            Err(_) | Ok(None) => break,
        }
    }
    // stop heartbeat
    let _ = tx.send(());

    println!("Disconnected");
    Ok(())
}
