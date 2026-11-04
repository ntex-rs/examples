//! Simple websocket client.
use std::{io, thread, time::Duration};

use futures::{SinkExt, StreamExt, channel::mpsc};
use ntex::{SharedCfg, rt, time, util::Bytes, ws};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[ntex::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=trace");
    }
    env_logger::init();

    let con = if std::env::args().any(|arg| arg == "--tls") {
        #[cfg(feature = "tls")]
        {
            use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};

            let mut config = SslConnector::builder(SslMethod::tls())?;
            config.set_verify(SslVerifyMode::PEER);
            config.set_ca_file("../../tls/certs/ca.pem")?;
            config.set_alpn_protos(b"\x08http/1.1")?;
            ws::WsClient::new("wss://localhost:8443/ws/", SharedCfg::default())
                .openssl(config.build())
                .connect()
                .await?
                .seal()
        }
        #[cfg(not(feature = "tls"))]
        {
            return Err("--tls requires --features tls".into());
        }
    } else {
        ws::WsClient::new("ws://127.0.0.1:8080/ws/", SharedCfg::default())
            .connect()
            .await?
            .seal()
    };

    println!("Got response: {:?}", con.response());

    let (mut tx, mut rx) = mpsc::unbounded();

    // start console read loop
    thread::spawn(move || {
        loop {
            let mut cmd = String::new();
            if io::stdin().read_line(&mut cmd).is_err() {
                println!("error");
                return;
            }

            // send text to server
            if futures::executor::block_on(tx.send(ws::Message::Text(cmd.into()))).is_err() {
                return;
            }
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
                    .map_err(io::Error::other)?;
            }
            Err(_) => break,
            _ => (),
        }
    }

    println!("Disconnected");
    Ok(())
}
