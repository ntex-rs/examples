# Run a server in a separate thread

Start an ntex server in a separate OS thread and control it from `main`.
`src/main.rs` sends a cloned `ntex::server::Server` handle back through a
`std::sync::mpsc` channel, then uses `Server::stop(true)` for graceful shutdown.

## Run the example

```sh
cd server/run-in-thread
cargo run
```

The server listens on `127.0.0.1:8080`. After receiving the server handle,
`main` waits ten seconds, requests graceful shutdown, and waits for it to
finish. The example sets `RUST_LOG=trace` before initializing the logger.

## Try it

Send a request during the ten-second window:

```sh
curl http://localhost:8080/
curl http://localhost:8080/index.html
```

Both routes return `Hello world!`. The process stops automatically; it is
not intended to keep serving indefinitely.

## Learn more

- [Controlling a running server](https://docs.ntex.rs/5-server.html#controlling-a-running-server)

## Tests

```sh
cargo test -p run-in-thread
```

Covers greeting routes, unknown routes, and prompt graceful shutdown on a separate server thread.
