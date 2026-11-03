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

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    ntex::server::build()
        .bind(
            "mqtt-websocket",
            "127.0.0.1:8081",
            SharedCfg::new("MQTT-WS").add(IoConfig::new().set_keepalive_timeout(Seconds::ZERO)),
            async |_| {
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
                                    Response::BadRequest()
                                        .body("The mqtt WebSocket subprotocol is required"),
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
            },
        )?
        .workers(1)
        .run()
        .await
}
