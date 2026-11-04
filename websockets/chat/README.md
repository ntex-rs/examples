# WebSocket chat

A multi-room chat server. Browsers and command-line clients connect over
WebSocket at `/ws/`; a single chat server task routes messages between them.

Things to look at:

- `src/main.rs` upgrades `GET /ws/` with `ws::start()` and runs one session per
  connection. A heartbeat task pings every 5 seconds and drops clients that
  stay silent for 10 seconds.
- `src/server.rs` is the chat server: a task that owns the room list and talks
  to the sessions over `futures` channels.
- The chat server handle reaches the handlers through application state
  (`ChatState`, `build_with()`, `to_with_state()`).

For a version that also accepts raw TCP clients, see
[`tcp-chat`](../tcp-chat/README.md).

## Run the server

```sh
cd websockets/chat
cargo run --bin websocket-chat-server
```

The server listens on `127.0.0.1:8080`. Run it from `websockets/chat` so the
static files in `static/` are found.

## Connect clients

**Browser:** open <http://localhost:8080/>, which redirects to
`/static/websocket.html`. Open it in several tabs to chat with yourself.

**Python:** requires [aiohttp](https://docs.aiohttp.org/):

```sh
pip install aiohttp
python client.py            # --host and --port default to 127.0.0.1:8080
```

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

- [Servers with `web::ws`](https://docs.ntex.rs/8.6-web-app.html#servers-with-webws)
  and [heartbeats and closing](https://docs.ntex.rs/8.6-web-app.html#heartbeats-and-closing-the-connection)
  in the WebSocket chapter of the ntex guide.
- [Application state](https://docs.ntex.rs/8.2-web-app.html#accessing-application-state-in-handlers).

## Tests

```sh
cargo test -p websocket-chat
```

The tests connect two clients to the real chat server and cover room broadcasts, ping/pong, and a missing room argument.
