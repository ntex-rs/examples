# ntex examples

A curated list of examples for [ntex](https://github.com/ntex-rs/ntex), a
framework for composable network services in Rust.

See also the [ntex website](https://ntex.rs) and the
[API documentation](https://docs.rs/ntex).

## Requirements

- Rust 1.97 or later
- Some examples need extra services (PostgreSQL, MySQL, SQLite, Redis) or
  OpenSSL development libraries; see the README in each example directory.

## Running an example

All examples are members of a single Cargo workspace and use ntex 4.
Change into an example directory and run it:

```sh
cd hello-world
cargo run
```

Then open <http://127.0.0.1:8080/> (most examples listen on port 8080).
Examples with multiple binaries (for example the websocket server and client)
are started with `cargo run --bin <name>`.

To build or test the whole workspace:

```sh
cargo build
cargo test
```

## Examples

### Basics

| Example | Description |
| --- | --- |
| [hello-world](hello-world) | Minimal web server with a unit test |
| [basics](basics) | Routing, path parameters, sessions, static files and error pages |
| [tokio](tokio) | The `basics` example running on the Tokio runtime |
| [async_ex1](async_ex1) | Chaining async steps, including outgoing HTTP client requests |
| [async_ex2](async_ex2) | Nested resource registration through application configuration |
| [error_handling](error_handling) | Custom error types and error propagation |
| [json_error](json_error) | Rendering errors as JSON responses |
| [middleware](middleware) | Writing and using custom middleware |
| [run-in-thread](run-in-thread) | Running the server in a separate thread and stopping it |
| [shutdown-server](shutdown-server) | Shutting down the server remotely or with a signal |
| [unix-socket](unix-socket) | Serving HTTP over a Unix domain socket |
| [docker_sample](docker_sample) | Building and running an ntex application in Docker |

### Application state

| Example | Description |
| --- | --- |
| [state1](state1) | Process-wide and per-worker application state |
| [state5](state5) | Per-connection state, including TLS connection info |

### Requests and responses

| Example | Description |
| --- | --- |
| [json](json) | JSON request and response handling |
| [jsonrpc](jsonrpc) | JSON-RPC over HTTP server |
| [form](form) | URL-encoded form handling |
| [multipart](multipart) | File uploads with multipart forms |
| [server-sent-events](server-sent-events) | Server-sent events (`EventSource`) |
| [static_index](static_index) | Serving static files with an index page |
| [http-proxy](http-proxy) | HTTP proxy forwarding requests to another server |

### Sessions and authentication

| Example | Description |
| --- | --- |
| [cookie-session](cookie-session) | Cookie-based sessions |
| [cookie-auth](cookie-auth) | Cookie-based identity and authentication |
| [simple-auth-server](simple-auth-server) | Registration and login with email verification (not in workspace) |

### Databases

| Example | Description |
| --- | --- |
| [async_db](async_db) | Running blocking SQLite queries asynchronously |
| [async_pg](async_pg) | PostgreSQL with `tokio-postgres` and `deadpool` |
| [r2d2](r2d2) | SQLite with an `r2d2` connection pool |
| [diesel](diesel) | Diesel with SQLite (not in workspace) |
| [todo](todo) | Todo application with Diesel, PostgreSQL and Tera templates |

### GraphQL

| Example | Description |
| --- | --- |
| [juniper](juniper) | GraphQL with Juniper |
| [graphql-demo](graphql-demo) | GraphQL with Juniper and MySQL |

### Templates

| Example | Description |
| --- | --- |
| [template_askama](template_askama) | Askama templates |
| [template_handlebars](template_handlebars) | Handlebars templates |
| [template_tera](template_tera) | Tera templates |
| [template_yarte](template_yarte) | Yarte templates |

### TLS

| Example | Description |
| --- | --- |
| [openssl](openssl) | HTTPS server with OpenSSL |
| [rustls](rustls) | HTTPS server with rustls |
| [awc_https](awc_https) | HTTPS requests with the ntex HTTP client |

### WebSockets

| Example | Description |
| --- | --- |
| [websocket](websocket) | WebSocket echo server and client |
| [websocket-lowlevel](websocket-lowlevel) | WebSocket server and client using the low-level API |
| [websocket-chat](websocket-chat) | WebSocket chat server |
| [websocket-tcp-chat](websocket-tcp-chat) | Chat server with WebSocket and TCP clients |

### Testing

| Example | Description |
| --- | --- |
| [test-mockall](test-mockall) | Testing a state-aware service with `mockall` |
| [test-shimforge](test-shimforge) | Testing a state-aware service with `shimforge` |

## License

This project is licensed under either of:

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
