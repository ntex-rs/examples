# ntex examples

[![Checks](https://github.com/ntex-rs/examples/actions/workflows/checks.yml/badge.svg)](https://github.com/ntex-rs/examples/actions/workflows/checks.yml)
[![CI (Linux)](https://github.com/ntex-rs/examples/actions/workflows/linux.yml/badge.svg)](https://github.com/ntex-rs/examples/actions/workflows/linux.yml)
[![CI (macOS)](https://github.com/ntex-rs/examples/actions/workflows/osx.yml/badge.svg)](https://github.com/ntex-rs/examples/actions/workflows/osx.yml)
[![CI (Windows)](https://github.com/ntex-rs/examples/actions/workflows/windows.yml/badge.svg)](https://github.com/ntex-rs/examples/actions/workflows/windows.yml)

A curated list of examples for [ntex](https://github.com/ntex-rs/ntex), a
framework for composable network services in Rust.

The examples are companions to the [ntex guide](https://docs.ntex.rs/): the
guide explains the concepts, and the examples show them in complete, runnable
applications. Each example README ends with links to the matching guide
sections. For individual types and functions, see the
[API documentation](https://docs.rs/ntex).

## Learn with the guide

New to ntex? Read the [introduction](https://docs.ntex.rs/) and the chapters
on [services](https://docs.ntex.rs/1-service.html) and
[pipelines](https://docs.ntex.rs/2-pipeline.html) first, then use the table
below to pair each chapter with examples.

| Guide chapter | Examples |
| --- | --- |
| [Applications and Routing](https://docs.ntex.rs/8.1-web-app.html) | [basics](basics), [extras/async-ex2](extras/async-ex2) |
| [Application State](https://docs.ntex.rs/8.2-web-app.html) | [state/web-state](state/web-state), [db/async-db](db/async-db), [db/r2d2](db/r2d2) |
| [Filters and Middleware](https://docs.ntex.rs/8.3-web-app.html) | [middleware](middleware), [extras/cookie-auth](extras/cookie-auth) |
| [Extractors and Responders](https://docs.ntex.rs/8.4-web-app.html) | [extractors/json](extractors/json), [extractors/form](extractors/form), [extractors/multipart](extractors/multipart) |
| [Error Handling](https://docs.ntex.rs/8.5-web-app.html) | [error](error), [extras/json-error](extras/json-error) |
| [WebSocket Support](https://docs.ntex.rs/8.6-web-app.html) | [websockets/basic](websockets/basic), [websockets/chat](websockets/chat), [websockets/lowlevel](websockets/lowlevel), [websockets/mqtt](websockets/mqtt) |
| [Server](https://docs.ntex.rs/5-server.html) | [server/shutdown](server/shutdown), [server/run-in-thread](server/run-in-thread), [server/unix-socket](server/unix-socket) |
| [Runtime](https://docs.ntex.rs/4-runtime.html) | [server/tokio](server/tokio), [db/async-db](db/async-db) |
| [Worker and Request State](https://docs.ntex.rs/6-server-app.html) | [state/process](state/process), [state/state-accumulation](state/state-accumulation) |
| [Service State](https://docs.ntex.rs/3-state.html#testing-state-aware-function-services) | [tests/mockall](tests/mockall), [tests/shimforge](tests/shimforge) |
| [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html) | [tls/x509](tls/x509), [tls/openssl](tls/openssl), [tls/rustls](tls/rustls), [tls/schannel](tls/schannel) |

The examples' tests follow the guide's testing advice; see
[testing routes](https://docs.ntex.rs/8.1-web-app.html#testing-routes),
[test servers](https://docs.ntex.rs/5-server.html#test-servers) and
[testing I/O](https://docs.ntex.rs/7-io.html#testing).

## Requirements

- Rust 1.97 or later
- Some examples need extra services or libraries (PostgreSQL and `libpq`,
  MySQL, SQLite, OpenSSL); see the README in each example directory.

## Running an example

All examples are members of a single Cargo workspace and use ntex 4.
Run an example by package name from the repository root, or change into its
directory and use `cargo run`:

```sh
cargo run -p basics
# or
cd basics && cargo run
```

Some examples read files relative to their own directory (certificates,
databases, static files), so their READMEs use the second form. Most examples listen on
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

Guide: [Applications and Routing](https://docs.ntex.rs/8.1-web-app.html), [Filters and Middleware](https://docs.ntex.rs/8.3-web-app.html), [Application State](https://docs.ntex.rs/8.2-web-app.html).

| Example | Description |
| --- | --- |
| [basics](basics) | Routing, path parameters, sessions, static files and error pages |
| [error](error) | Custom error domain rendered as uniform JSON responses |
| [middleware](middleware) | Writing middleware: body logging, redirects and the middleware life cycle |
| [state/process](state/process) | Process-wide and per-worker application state |
| [state/state-accumulation](state/state-accumulation) | Per-connection state, including TLS connection info |
| [state/web-state](state/web-state) | `web::App` state in middleware, filters, handlers and errors |

### Server

Guide: [Server](https://docs.ntex.rs/5-server.html) and [Runtime](https://docs.ntex.rs/4-runtime.html).

| Example | Description |
| --- | --- |
| [server/tokio](server/tokio) | The `basics` example running on the Tokio runtime |
| [server/run-in-thread](server/run-in-thread) | Running the server in a separate thread and stopping it |
| [server/shutdown](server/shutdown) | Shutting down the server remotely or with a signal |
| [server/unix-socket](server/unix-socket) | Serving HTTP over a Unix domain socket |
| [server/docker](server/docker) | Building and running an ntex application in Docker |

### Extractors

Guide: [Extractors and Responders](https://docs.ntex.rs/8.4-web-app.html).

| Example | Description |
| --- | --- |
| [extractors/json](extractors/json) | JSON request and response handling |
| [extractors/form](extractors/form) | URL-encoded form handling |
| [extractors/multipart](extractors/multipart) | File uploads with multipart forms |

### Databases

Guide: [Application State](https://docs.ntex.rs/8.2-web-app.html) and [blocking work](https://docs.ntex.rs/4-runtime.html#blocking-work).

| Example | Description |
| --- | --- |
| [db/async-db](db/async-db) | Running blocking SQLite queries asynchronously |
| [db/async-pg](db/async-pg) | PostgreSQL with `tokio-postgres` and `deadpool` |
| [db/r2d2](db/r2d2) | SQLite with an `r2d2` connection pool |
| [db/diesel](db/diesel) | Diesel with SQLite |
| [db/juniper](db/juniper) | GraphQL with Juniper |
| [db/graphql](db/graphql) | GraphQL with Juniper and MySQL |

### Templates

Guide: [Extractors and Responders](https://docs.ntex.rs/8.4-web-app.html#responders).

| Example | Description |
| --- | --- |
| [templates/askama](templates/askama) | Askama templates |
| [templates/handlebars](templates/handlebars) | Handlebars templates |
| [templates/tera](templates/tera) | Tera templates |
| [templates/yarte](templates/yarte) | Yarte templates |

### TLS

| Example | Description |
| --- | --- |
| [tls/openssl](tls/openssl) | OpenSSL HTTPS client/server and mutual authentication |
| [tls/rustls](tls/rustls) | rustls HTTPS client/server, providers and mutual authentication |
| [tls/schannel](tls/schannel) | Windows TLS, store identities and explicit client authentication |
| [tls/x509](tls/x509) | Portable identity loading, certificate inspection and peer authorization |

Start with the [local certificate setup](tls/README.md), and read
[Securing connections with TLS](https://docs.ntex.rs/10-tls.html) in the guide.

### WebSockets

Guide: [WebSocket Support](https://docs.ntex.rs/8.6-web-app.html).

| Example | Description |
| --- | --- |
| [websockets/basic](websockets/basic) | WebSocket echo server and client, over plain TCP or TLS |
| [websockets/lowlevel](websockets/lowlevel) | WebSocket server and client using the low-level API |
| [websockets/mqtt](websockets/mqtt) | MQTT server and client over a binary WebSocket transport |
| [websockets/chat](websockets/chat) | Multi-room WebSocket chat server with a browser client |
| [websockets/tcp-chat](websockets/tcp-chat) | Chat server with WebSocket and TCP clients |

### Testing

Guide: [Testing state-aware function services](https://docs.ntex.rs/3-state.html#testing-state-aware-function-services) and [testing routes](https://docs.ntex.rs/8.1-web-app.html#testing-routes).

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
