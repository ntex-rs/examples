//! Publish one MQTT message through a WebSocket connection.
use std::{io, rc::Rc};

use ntex::connect::{Connect, ConnectError};
use ntex::http::header;
use ntex::{Pipeline, SharedCfg, time::Seconds, time::timeout, util::Bytes, ws};
use ntex_mqtt::v3;

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    timeout(Seconds(10), publish())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "MQTT example timed out"))?
}

async fn publish() -> io::Result<()> {
    let cfg: SharedCfg = SharedCfg::new("MQTT-WS")
        .add(
            ws::WsClientConfig::new()
                .set_protocols(["mqtt"])
                .map_err(io::Error::other)?,
        )
        .into();
    let websocket = Rc::new(ws::WsClient::new("ws://127.0.0.1:8081/mqtt", cfg.clone()));

    let client = Pipeline::new(
        cfg,
        v3::client::MqttConnector::new().connector(move |_: Connect<&str>| {
            let websocket = websocket.clone();
            async move {
                let connection = websocket
                    .connect()
                    .await
                    .map_err(|err| ConnectError::Io(io::Error::other(err)))?;
                if connection
                    .response()
                    .headers()
                    .get(header::SEC_WEBSOCKET_PROTOCOL)
                    != Some(&header::HeaderValue::from_static("mqtt"))
                {
                    return Err(ConnectError::Io(io::Error::other(
                        "The server did not select the mqtt subprotocol",
                    ))
                    .into());
                }
                Ok(connection.into_transport())
            }
        }),
    )
    .call(
        v3::client::Connect::new("127.0.0.1:8081")
            .client_id("websocket-example")
            .keep_alive(Seconds(10)),
    )
    .await
    .map_err(io::Error::other)?;

    let sink = client.sink();
    let dispatcher = ntex::rt::spawn(client.start_default());
    let result = sink
        .publish("example/hello")
        .send_at_least_once(Bytes::from_static(b"MQTT over WebSocket"))
        .await;
    sink.close();
    dispatcher.await.map_err(io::Error::other)?;
    result.map_err(io::Error::other)?;
    println!("Publish acknowledged; connection closed");
    Ok(())
}
