//! Simple websocket client.
use std::time::Duration;
use std::{io, thread};

use bytes::Bytes;
use futures::channel::mpsc;
use futures::SinkExt;
use ntex::http::client::{ws, Client};
use ntex::rt;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// Websockets handler service
async fn service(frame: ws::Frame) -> Result<Option<ws::Message>, io::Error> {
    match frame {
        ws::Frame::Text(text) => {
            println!("Server: {:?}", text);
        }
        ws::Frame::Ping(msg) => {
            // send pong response
            println!("Got server ping: {:?}", msg);
            return Ok(Some(ws::Message::Pong(msg)));
        }
        _ => (),
    }
    Ok(None)
}

#[ntex::main]
async fn main() {
    std::env::set_var("RUST_LOG", "ntex=trace");
    env_logger::init();

    // open websockets connection over http transport
    let (response, framed) = Client::new()
        .ws("http://127.0.0.1:8080/ws/")
        .connect()
        .await
        .unwrap();

    println!("Got response: {:?}", response);

    let (mut tx, rx) = mpsc::unbounded();

    // start console read loop
    let mut tx2 = tx.clone();
    thread::spawn(move || loop {
        let mut cmd = String::new();
        if io::stdin().read_line(&mut cmd).is_err() {
            println!("error");
            return;
        }

        // send text to server
        if futures::executor::block_on(tx2.send(ws::Message::Text(cmd))).is_err() {
            return;
        }
    });

    // start heartbeat task
    rt::spawn(async move {
        rt::time::delay_for(HEARTBEAT_INTERVAL).await;
        // send ping
        if tx.send(ws::Message::Ping(Bytes::new())).await.is_err() {
            return;
        }
    });

    // run ws protocol dispatcher
    let _ = ws::start(framed, rx, service).await;

    println!("Disconnected");
}
