# WebSocket and TCP chat

The [WebSocket chat](../chat/README.md) extended with a second protocol. One
ntex server has two listeners that share a single chat server, so WebSocket
and raw TCP users can chat in the same rooms:

| Listener | Address | Protocol |
|----------|---------|----------|
| `websockets` | `127.0.0.1:8080` | HTTP and WebSocket (`/ws/`), plus the browser client |
| `tcp` | `127.0.0.1:12345` | Custom framing: a 2-byte big-endian length, then a JSON message |

Things to look at:

- `src/main.rs` uses `ntex::server::build()` with two `bind()` calls, one per
  protocol.
- `src/web.rs` is the WebSocket endpoint and `src/tcp.rs` the TCP endpoint.
  Both send heartbeats every 5 seconds and drop silent clients after 10
  seconds.
- `src/codec.rs` implements the TCP framing with ntex's `Decoder` and
  `Encoder` traits.
- `src/server.rs` is the shared chat server that owns the rooms.

## Run the server

```sh
cd websockets/tcp-chat
cargo run --bin websocket-tcp-chat-server
```

Run it from `websockets/tcp-chat` so the static files in `static/` are found.

## Connect clients

Start each client in its own terminal, from `websockets/tcp-chat`:

```sh
cargo run --bin websocket-tcp-chat-client      # WebSocket, ws://127.0.0.1:8080/ws/
cargo run --bin websocket-tcp-chat-tcp-client  # TCP, 127.0.0.1:12345
```

Both read lines from stdin and print what the server sends back.

**Browser:** open <http://localhost:8080/>, which redirects to
`/static/websocket.html`.

**Python:** `pip install aiohttp`, then `python client.py` (WebSocket).

## Chat commands

Every new session joins the `Main` room. Lines that start with `/` are
commands; anything else is sent to everyone else in your current room.

| Command | Effect |
|---------|--------|
| `/list` | List the available rooms |
| `/join <room>` | Join a room, creating it if needed |
| `/name <name>` | Set the name shown before your messages |
| any other text | Send a message to the current room |

## Learn more

- [Binding and existing listeners](https://docs.ntex.rs/5-server.html#binding-and-existing-listeners)
  in the Server chapter of the ntex guide.
- [Servers with `web::ws`](https://docs.ntex.rs/8.6-web-app.html#servers-with-webws)
  in the WebSocket chapter.

## Tests

```sh
cargo test -p chat-example
```

The tests cover broadcasts between two WebSocket clients and heartbeat and room-list replies over the real TCP protocol.
