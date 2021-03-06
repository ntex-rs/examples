//! Simple websocket client.
use std::{io, thread, time::Duration};

use bytes::Bytes;
use futures::{channel::mpsc, SinkExt, StreamExt};
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
    let con = Client::new()
        .ws("http://127.0.0.1:8080/ws/")
        .connect()
        .await
        .unwrap();

    println!("Got response: {:?}", con.response());

    let (mut tx, mut rx) = mpsc::unbounded();

    // start console read loop
    thread::spawn(move || loop {
        let mut cmd = String::new();
        if io::stdin().read_line(&mut cmd).is_err() {
            println!("error");
            return;
        }

        // send text to server
        if futures::executor::block_on(tx.send(ws::Message::Text(cmd.into()))).is_err() {
            return;
        }
    });

    // read console commands
    let sink = con.sink();
    rt::spawn(async move {
        while let Some(msg) = rx.next().await {
            if sink.send(msg).await.is_err() {
                return;
            }
        }
    });

    // start heartbeat task
    let sink = con.sink();
    rt::spawn(async move {
        rt::time::delay_for(HEARTBEAT_INTERVAL).await;
        // send ping
        if sink.send(ws::Message::Ping(Bytes::new())).await.is_err() {
            return;
        }
    });

    // run ws protocol dispatcher
    let _ = con.start::<_, _, ()>(service).await;

    println!("Disconnected");
}
