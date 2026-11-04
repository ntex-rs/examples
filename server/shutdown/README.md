# Shut down a server with a request

Use application state and a channel to stop an ntex HTTP server from a
handler. In `src/main.rs`, `web::AppState` holds the shutdown sender; a
control thread receives the message and awaits `Server::stop(true)`.

## Run the example

```sh
cd server/shutdown
cargo run --bin shutdown-server
```

Both the package and its binary are named `shutdown-server`. The server
listens on `127.0.0.1:8080` and sets `RUST_LOG=debug`. The worker count uses
the ntex default; it is not fixed at eight.

## Try it

| Request | Result |
| --- | --- |
| `GET /hello` | `Hello world!` |
| `POST /stop` | `204 No Content` and a request to stop the server gracefully |

```sh
curl http://localhost:8080/hello
curl -i -X POST http://localhost:8080/stop
```

You can also stop the process with Ctrl-C (`SIGINT`). Unlike the `/stop`
handler, the default `SIGINT` handling stops the server immediately.

## Learn more

- [Controlling a running server](https://docs.ntex.rs/5-server.html#controlling-a-running-server)
- [Signals and shutdown](https://docs.ntex.rs/5-server.html#signals-and-shutdown)

## Tests

```sh
cargo test -p shutdown-server
```

Covers greetings, shutdown notifications, rejected methods, and graceful server shutdown.
