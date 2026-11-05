//! MQTT 3.1.1 over a binary WebSocket transport.
use std::{io, rc::Rc};

use ntex::http::{HttpService, Response, body::BodySize, h1, header};
use ntex::io::{IoBoxed, IoConfig};
use ntex::{Pipeline, Service, SharedCfg, time::Seconds, ws};
use ntex_mqtt::{MqttError, v3};

fn mqtt_service() -> impl Service<(), IoBoxed, Res = (), Error = io::Error> {
    v3::MqttServer::new(async |publish: v3::Publish| {
        let payload = publish.read_all().await.map_err(io::Error::other)?;
        println!("{}: {payload:?}", publish.topic().path());
        Ok::<_, io::Error>(())
    })
    .build(async |connect: v3::Connect| Ok::<_, io::Error>(connect.ack((), false)))
    .map_err(|err: MqttError<io::Error>| io::Error::other(format!("{err:?}")))
}

/// Builds the MQTT WebSocket upgrade service.
fn server() -> impl Service<(), ntex::io::Io, Res = (), Error = ntex::http::error::DispatchError> {
    let mqtt = Rc::new(Pipeline::new((), mqtt_service()));

    HttpService::h1(async |_| Ok::<_, io::Error>(Response::NotFound().build())).control(
        move |msg: h1::Control<_, _>| {
            let mqtt = mqtt.clone();
            async move {
                let h1::Control::Upgrade(upgrade) = msg else {
                    return Ok::<_, io::Error>(msg.ack());
                };

                let req = upgrade.get_ref();
                if req.path() != "/mqtt" {
                    return Ok(upgrade.fail_with(Response::NotFound().build()));
                }
                let mut response = match ws::handshake(req.head()) {
                    Ok(response) => response,
                    Err(err) => return Ok(upgrade.fail(err)),
                };
                let mqtt_offered = req
                    .headers()
                    .get_all(header::SEC_WEBSOCKET_PROTOCOL)
                    .filter_map(|value| value.to_str().ok())
                    .flat_map(|value| value.split(','))
                    .any(|protocol| protocol.trim() == "mqtt");
                if !mqtt_offered {
                    return Ok(upgrade.fail_with(
                        Response::BadRequest().body("The mqtt WebSocket subprotocol is required"),
                    ));
                }
                response.set_header(header::SEC_WEBSOCKET_PROTOCOL, "mqtt");

                let (ack, io, _, codec) = upgrade.handle();
                io.stop_timer();
                io.send(
                    h1::Message::Item((response.build().drop_body(), BodySize::Empty)),
                    &codec,
                )
                .await?;

                let io = ws::WsTransport::create(io, ws::Codec::new());
                mqtt.call(io.boxed()).await?;
                Ok(ack)
            }
        },
    )
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    ntex::server::build()
        .bind(
            "mqtt-websocket",
            "127.0.0.1:8081",
            SharedCfg::new("MQTT-WS").add(IoConfig::new().set_keepalive_timeout(Seconds::ZERO)),
            async |_| server(),
        )?
        .workers(1)
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{
        http::{StatusCode, test},
        time::{Seconds, timeout},
        util::Bytes,
    };

    /// STATEMENT: MQTT connects and answers a heartbeat over binary WebSocket frames.
    ///
    /// HOW:
    /// - Connect to the real `/mqtt` service offering the `mqtt` subprotocol and check its selection.
    /// - Send MQTT 3.1.1 `CONNECT` and `PINGREQ` packets and check `CONNACK` and `PINGRESP` within a timeout.
    #[ntex::test]
    async fn mqtt_connects_and_answers_ping() {
        let srv = test::server(async |_| server());
        timeout(Seconds(3), async {
            let cfg = SharedCfg::new("MQTT-TEST")
                .add(ws::WsClientConfig::new().set_protocols(["mqtt"]).unwrap());
            let connection = ws::WsClient::new(srv.url("/mqtt"), cfg)
                .connect()
                .await
                .unwrap();
            assert_eq!(
                connection
                    .response()
                    .headers()
                    .get(header::SEC_WEBSOCKET_PROTOCOL)
                    .unwrap(),
                "mqtt"
            );
            let (io, codec, _) = connection.into_inner();
            io.send(
                ws::Message::Binary(Bytes::from_static(
                    b"\x10\x10\x00\x04MQTT\x04\x02\x00\x00\x00\x04test",
                )),
                &codec,
            )
            .await
            .unwrap();
            assert_eq!(
                io.recv(&codec).await.unwrap().unwrap(),
                ws::Frame::Binary(Bytes::from_static(b"\x20\x02\x00\x00"))
            );
            io.send(ws::Message::Binary(Bytes::from_static(b"\xc0\x00")), &codec)
                .await
                .unwrap();
            assert_eq!(
                io.recv(&codec).await.unwrap().unwrap(),
                ws::Frame::Binary(Bytes::from_static(b"\xd0\x00"))
            );
        })
        .await
        .unwrap();
    }

    /// STATEMENT: MQTT requires its WebSocket subprotocol and rejects unknown paths.
    ///
    /// HOW:
    /// - Use `ws_at` without offering `mqtt` and check for `400 Bad Request`.
    /// - Request another path and check for `404 Not Found`.
    #[ntex::test]
    async fn mqtt_rejects_missing_subprotocol_and_unknown_path() {
        let srv = test::server(async |_| server());
        timeout(Seconds(3), async {
            let error = srv.ws_at("/mqtt").await.err().unwrap();
            assert!(matches!(
                *error,
                ws::error::WsClientError::InvalidResponseStatus(StatusCode::BAD_REQUEST)
            ));
            let response = srv
                .request(ntex::http::Method::GET, "/missing")
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        })
        .await
        .unwrap();
    }
}
