# MQTT over WebSocket

A minimal MQTT 3.1.1 publish server and client using WebSocket as a binary
transport. This is a publish consumer, not a broker: it prints incoming
payloads and acknowledges QoS 1 publishes, but does not implement subscriptions
or forwarding.

## Run

From the examples repository, start the server:

```sh
cd protocols/mqtt
cargo run --bin websocket-mqtt-server
```

In another terminal:

```sh
cd protocols/mqtt
cargo run --bin websocket-mqtt-client
```

The client connects to `ws://127.0.0.1:8081/mqtt`, publishes
`MQTT over WebSocket` on `example/hello`, waits for the MQTT acknowledgement,
and closes. It has a ten-second overall timeout.

## How it works

The HTTP/1 control service accepts upgrades only on `/mqtt`. It validates the
WebSocket handshake and requires the `mqtt` subprotocol, returning an HTTP
error otherwise. After sending `101 Switching Protocols`, it installs
`WsTransport` and passes the resulting I/O stream to `ntex_mqtt::v3::MqttServer`.
The inherited HTTP timer is stopped; MQTT manages its own keepalive.

The client offers `mqtt`, checks that the server selected it, and returns
`WsConnection::into_transport()` from a custom `MqttConnector` connector.
The MQTT dispatcher runs concurrently with the publish so it can receive
the acknowledgement.

`WsTransport` carries binary bytes, not MQTT packet boundaries. MQTT packets
may span WebSocket messages, and a message may contain multiple packets.
The filter handles WebSocket ping/pong and close frames and rejects text.
MQTT CONNECT, packet decoding, acknowledgements, and keepalive remain the
MQTT service's responsibility.

This loopback demonstration uses unencrypted `ws://` and accepts every MQTT
client. A deployed service needs authentication, an appropriate HTTP Origin
policy for browser clients, payload limits, and TLS (`wss://`).

## Tests

```sh
cargo test -p websocket-mqtt
```

The tests cover an MQTT connection and heartbeat over binary WebSocket frames, required subprotocol negotiation, and unknown paths.

## Learn more

- [WebSocket as a byte transport](https://docs.ntex.rs/8.6-web-app.html#websocket-as-a-byte-transport)
- [MQTT over WebSocket](https://docs.ntex.rs/8.6-web-app.html#mqtt-over-websocket)
- [Choosing a subprotocol](https://docs.ntex.rs/8.6-web-app.html#choosing-a-subprotocol)
- [Low-level servers with an HTTP control service](https://docs.ntex.rs/8.6-web-app.html#low-level-servers-with-an-http-control-service)
