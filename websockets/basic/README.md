# websocket

Simple echo websocket server.

The frame handlers are independent of TLS. Only connection setup changes
between the plain and secure variants below.

## Usage

### server

```bash
cd websockets/basic
cargo run --bin websocket-server
# Started http server: 127.0.0.1:8080
```

### web client

- [http://localhost:8080/index.html](http://localhost:8080/index.html)

### rust client

```bash
cd websockets/basic
cargo run --bin websocket-client
```

### python client

- ``pip install aiohttp``
- ``python websocket-client.py``

if ubuntu :

- ``pip3 install aiohttp``
- ``python3 websocket-client.py``

## Secure WebSocket (`wss://`)

Generate certificates with `sh tls/generate-certs.sh` from the repository root.
Then run from `websockets/basic`, in separate terminals:

```sh
cargo run --features tls --bin websocket-server -- --tls
cargo run --features tls --bin websocket-client -- --tls
```

The server listens on `127.0.0.1:8443`. Type a line in the client terminal to
receive an echo. The optional `tls` feature enables OpenSSL; the client trusts
`../../tls/certs/ca.pem`, verifies the hostname, and offers only `http/1.1` for
the WebSocket upgrade. It explicitly installs a TLS connector: changing the
URL to `wss://` alone is not sufficient.

Both client branches erase the filter type with `seal()` before entering the
same message-handling loop. The server uses the same upgrade and frame handlers
with either `bind` or `bind_openssl`. See the [TLS examples](../../tls/README.md)
for backend configuration and client-certificate authentication.
