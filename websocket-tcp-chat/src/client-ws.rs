//! Simple websocket client.
use std::{io, thread, time::Duration};

use futures::{channel::mpsc, SinkExt, StreamExt};
use ntex::{rt, time, util::Bytes, ws};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[ntex::main]
async fn main() -> Result<(), io::Error> {
    std::env::set_var("RUST_LOG", "ntex=trace");
    env_logger::init();

    // open websockets connection over http transport
    let con = ws::WsClient::build("http://127.0.0.1:8080/ws/")
        .finish()
        .unwrap()
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
        loop {
            time::sleep(HEARTBEAT_INTERVAL).await;
            if sink.send(ws::Message::Ping(Bytes::new())).await.is_err() {
                return;
            }
        }
    });

    // run ws dispatcher
    let sink = con.sink();
    let mut rx = con.seal().receiver();

    while let Some(frame) = rx.next().await {
        match frame {
            Ok(ws::Frame::Text(text)) => {
                println!("Server: {:?}", text);
            }
            Ok(ws::Frame::Ping(msg)) => {
                // send pong response
                println!("Got server ping: {:?}", msg);
                sink.send(ws::Message::Pong(msg))
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }
            Err(_) => break,
            _ => (),
        }
    }

    println!("Disconnected");
    Ok(())
}
