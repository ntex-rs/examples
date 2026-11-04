# WebSocket echo server

A minimal WebSocket server that echoes back every message it receives, plus
three clients: a browser page, a Rust program, and a Python script.

The same frame handlers serve plain `ws://` and encrypted `wss://`
connections. Only the connection setup changes; see
[Secure WebSocket](#secure-websocket-wss) below.

## Run the server

```sh
cd websockets/basic
cargo run --bin websocket-server
```

The server listens on `127.0.0.1:8080`.

## Connect a client

**Browser:** open <http://localhost:8080/index.html>.

**Rust:** in another terminal:

```sh
cd websockets/basic
cargo run --bin websocket-client
```

Type a line and press Enter to see it echoed back.

**Python:** requires [aiohttp](https://docs.aiohttp.org/):

```sh
pip install aiohttp        # or: pip3 install aiohttp
python websocket-client.py # or: python3 websocket-client.py
```

## Secure WebSocket (`wss://`)

First create the local certificates from the repository root:

```sh
sh tls/generate-certs.sh
```

Then, from `websockets/basic`, start the server and client in separate
terminals:

```sh
cargo run --features tls --bin websocket-server -- --tls
cargo run --features tls --bin websocket-client -- --tls
```

The secure server listens on `127.0.0.1:8443`. As before, type a line in the
client terminal to get an echo. The `tls` feature enables OpenSSL.

Three details in the client are worth noticing:

- **It installs a TLS connector explicitly.** `WsClient` uses plain TCP by
  default, so changing the URL to `wss://` alone does not enable TLS.
- **It verifies the server.** It trusts `../../tls/certs/ca.pem` and checks
  the hostname, so no verification is turned off.
- **It offers only `http/1.1` through ALPN.** The WebSocket handshake is an
  HTTP/1.1 upgrade.

Both client branches call `seal()` to hide the connection's filter type. That
lets the plain and TLS connections share one message loop. On the server side,
the only difference is `bind` versus `bind_openssl`.

For other backends and client-certificate authentication, see the
[TLS examples](../../tls/README.md).

## Learn more

- The guide chapter [WebSocket Support](https://docs.ntex.rs/8.6-web-app.html) covers
  [servers with `web::ws`](https://docs.ntex.rs/8.6-web-app.html#servers-with-webws),
  [heartbeats and closing](https://docs.ntex.rs/8.6-web-app.html#heartbeats-and-closing-the-connection), and
  [clients](https://docs.ntex.rs/8.6-web-app.html#clients-with-ntexws).
- [Configuration and TLS](https://docs.ntex.rs/8.6-web-app.html#configuration-and-tls) and the TLS chapter's
  [WebSocket client](https://docs.ntex.rs/10-tls.html#websocket-client) section explain `wss://`.

## Tests

```sh
cargo test -p websocket
```

The tests exercise text echo, ping/pong, and rejection of non-GET requests through the real routes.

Run `cargo test -p websocket --features tls` to also build and test with TLS support enabled.
