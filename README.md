# ntex examples

A curated list of examples for [ntex](https://github.com/ntex-rs/ntex), a
framework for composable network services in Rust.

See also the [ntex website](https://ntex.rs) and the
[API documentation](https://docs.rs/ntex).

## Requirements

- Rust 1.97 or later
- Some examples need extra services or libraries (PostgreSQL and `libpq`,
  MySQL, SQLite, OpenSSL); see the README in each example directory.

## Running an example

All examples are members of a single Cargo workspace and use ntex 4.
Change into an example directory and run it:

```sh
cd basics
cargo run
```

Some examples read static files or templates relative to their own
directory, so run them from there. Most examples listen on
<http://127.0.0.1:8080/>; check the example's README or `main.rs` for the
address. Examples with multiple binaries (for example the websocket server and
client) are started with `cargo run --bin <name>`.

To build or test the whole workspace:

```sh
cargo build
cargo test
```

Building `extras/todo` and `extras/simple-auth-server` requires the
PostgreSQL client library (`libpq`).

## Examples

### Basics

| Example | Description |
| --- | --- |
| [basics](basics) | Routing, path parameters, sessions, static files and error pages |
| [error](error) | Custom error types and error propagation |
| [middleware](middleware) | Writing and using custom middleware |
| [state1](state1) | Process-wide and per-worker application state |
| [state5](state5) | Per-connection state, including TLS connection info |

### Server

| Example | Description |
| --- | --- |
| [server/tokio](server/tokio) | The `basics` example running on the Tokio runtime |
| [server/run-in-thread](server/run-in-thread) | Running the server in a separate thread and stopping it |
| [server/shutdown](server/shutdown) | Shutting down the server remotely or with a signal |
| [server/unix-socket](server/unix-socket) | Serving HTTP over a Unix domain socket |
| [server/docker](server/docker) | Building and running an ntex application in Docker |

### Extractors

| Example | Description |
| --- | --- |
| [extractors/json](extractors/json) | JSON request and response handling |
| [extractors/form](extractors/form) | URL-encoded form handling |
| [extractors/multipart](extractors/multipart) | File uploads with multipart forms |

### Databases

| Example | Description |
| --- | --- |
| [db/async-db](db/async-db) | Running blocking SQLite queries asynchronously |
| [db/async-pg](db/async-pg) | PostgreSQL with `tokio-postgres` and `deadpool` |
| [db/r2d2](db/r2d2) | SQLite with an `r2d2` connection pool |
| [db/diesel](db/diesel) | Diesel with SQLite |
| [db/juniper](db/juniper) | GraphQL with Juniper |
| [db/graphql](db/graphql) | GraphQL with Juniper and MySQL |

### Templates

| Example | Description |
| --- | --- |
| [templates/askama](templates/askama) | Askama templates |
| [templates/handlebars](templates/handlebars) | Handlebars templates |
| [templates/tera](templates/tera) | Tera templates |
| [templates/yarte](templates/yarte) | Yarte templates |

### TLS

| Example | Description |
| --- | --- |
| [tls/openssl](tls/openssl) | HTTPS server with OpenSSL |
| [tls/rustls](tls/rustls) | HTTPS server with rustls |

### WebSockets

| Example | Description |
| --- | --- |
| [websockets/basic](websockets/basic) | WebSocket echo server and client |
| [websockets/lowlevel](websockets/lowlevel) | WebSocket server and client using the low-level API |
| [websockets/chat](websockets/chat) | WebSocket chat server |
| [websockets/tcp-chat](websockets/tcp-chat) | Chat server with WebSocket and TCP clients |

### Testing

| Example | Description |
| --- | --- |
| [tests/mockall](tests/mockall) | Testing a state-aware service with `mockall` |
| [tests/shimforge](tests/shimforge) | Testing a state-aware service with `shimforge` |

### Extras

| Example | Description |
| --- | --- |
| [extras/async-ex1](extras/async-ex1) | Chaining async steps, including outgoing HTTP client requests |
| [extras/async-ex2](extras/async-ex2) | Nested resource registration through application configuration |
| [extras/http-client](extras/http-client) | HTTPS requests with the ntex HTTP client |
| [extras/http-proxy](extras/http-proxy) | HTTP proxy forwarding requests to another server |
| [extras/json-error](extras/json-error) | Rendering errors as JSON responses |
| [extras/jsonrpc](extras/jsonrpc) | JSON-RPC over HTTP server |
| [extras/server-sent-events](extras/server-sent-events) | Server-sent events (`EventSource`) |
| [extras/cookie-session](extras/cookie-session) | Cookie-based sessions |
| [extras/cookie-auth](extras/cookie-auth) | Cookie-based identity and authentication |
| [extras/simple-auth-server](extras/simple-auth-server) | Registration and login with email invitations, Diesel and PostgreSQL |
| [extras/todo](extras/todo) | Todo application with Diesel, PostgreSQL and Tera templates |

## License

This project is licensed under either of:

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
