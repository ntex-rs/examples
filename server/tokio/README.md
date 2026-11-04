# Web basics on the Tokio runtime

A tour of ntex routing, path extractors, cookie sessions, streamed responses,
static files, redirects, and error responses on Tokio. `src/main.rs` uses
`#[tokio::main]` and runs the ntex application with
`ntex::rt::System::run_local`; `Cargo.toml` enables the `tokio` feature on
`ntex` and the `full` feature on `tokio`.

## Run the example

```sh
cd server/tokio
cargo run
```

The server listens on `127.0.0.1:8080`. `NamedFile` and `Files` resolve
`static/` relative to `CARGO_MANIFEST_DIR`, so launching from the workspace
root also works. The example sets `RUST_LOG=ntex=info` before initializing
the logger.

## Try it

| Request | Result |
| --- | --- |
| [`GET /`](http://localhost:8080/) | `302 Found` with `Location: static/welcome.html` |
| [`GET /welcome`](http://localhost:8080/welcome) | HTML welcome page; updates a counter in the cookie session |
| [`GET /favicon`](http://localhost:8080/favicon) | `static/favicon.ico`, served with `NamedFile` |
| [`GET /static/`](http://localhost:8080/static/) | Directory listing; files are available under `/static/<file>` |
| [`GET /static/welcome.html`](http://localhost:8080/static/welcome.html) | Static welcome page, without invoking the `/welcome` handler |
| [`GET /user/bob`](http://localhost:8080/user/bob) | `Hello bob!` as `text/plain` |
| [`GET /files/a/b/c`](http://localhost:8080/files/a/b/c) | `it's matching !`; logs the captured path |
| [`GET /async-body/bob`](http://localhost:8080/async-body/bob) | `Hello bob!` as a streaming response body |
| `GET /test`, `POST /test`, other methods on `/test` | `200 OK`, `405 Method Not Allowed`, `404 Not Found`, respectively |
| [`GET /error`](http://localhost:8080/error) | `500 Internal Server Error` with body `test`, returned through `InternalError`; does not panic |
| `GET` on an unmatched path, such as `/not-found` | `404 Not Found` with the page from `static/404.html` |
| Non-`GET` on an unmatched path | `405 Method Not Allowed` |

The session counter is stored and logged, but not inserted into the HTML.
The root redirect goes to the static page, not the session handler. The
session key is a fixed example key and secure cookies are disabled; this
configuration is for local demonstration, not production.

```sh
curl -i http://localhost:8080/
curl -i -X POST http://localhost:8080/test
curl -i http://localhost:8080/error
```

## Learn more

- [Runtime](https://docs.ntex.rs/4-runtime.html)
- [Applications and routing](https://docs.ntex.rs/8.1-web-app.html)
- [Error handling](https://docs.ntex.rs/8.5-web-app.html)

## Tests

Run from the workspace root (the path-qualified package avoids ambiguity with the Tokio dependency).

```sh
cargo test -p path+file://"$(pwd)/server/tokio"
```

Covers sessions, routing, streaming bodies, static files, redirects, and HTTP error responses in process.
