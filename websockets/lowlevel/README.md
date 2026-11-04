# Low-level WebSocket server

A TLS echo server that handles `h1::Control::Upgrade`, sends the WebSocket
handshake response with the HTTP codec, and then reads and writes WebSocket
frames directly. Ordinary HTTP requests still go to the web application.

The example replies to pings, tracks pongs for its heartbeat, and echoes close
frames. It rejects fragmented messages rather than assembling them.

## Usage

### server

```bash
cd websockets/lowlevel
cargo run --bin websocket-lowlevel-server
# Started HTTPS server: 127.0.0.1:8080
```

### web client

- [https://localhost:8080/index.html](https://localhost:8080/index.html)

The server uses the test certificate and key in `tls/openssl`. Run it from
`websockets/lowlevel` so the relative certificate and static-file paths resolve.
The browser will warn about the self-signed test certificate.

### rust client

```bash
cd websockets/lowlevel
cargo run --bin websocket-lowlevel-client
```

The Rust client connects to `wss://127.0.0.1:8080/ws/`. It disables certificate
verification only for this local self-signed demonstration. Production clients
must validate the server certificate and hostname.

## Tests

```sh
cargo test -p websocket-lowlevel
```

The tests exercise the real HTTP upgrade service with text echo, ping/pong, and a missing HTTP path.

## Learn more

- [Low-level servers with an HTTP control service](https://docs.ntex.rs/8.6-web-app.html#low-level-servers-with-an-http-control-service)
- [Frames, messages, and fragmentation](https://docs.ntex.rs/8.6-web-app.html#frames-messages-and-fragmentation)
- [Heartbeats and closing the connection](https://docs.ntex.rs/8.6-web-app.html#heartbeats-and-closing-the-connection)
- [Clients with `ntex::ws`](https://docs.ntex.rs/8.6-web-app.html#clients-with-ntexws) and [TLS](https://docs.ntex.rs/10-tls.html#websocket-client)
