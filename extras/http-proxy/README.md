# HTTP forwarding proxy

This example forwards requests to a single HTTP upstream with
`ntex::client::Client`. The `forward` handler in `src/main.rs` preserves the
request path and query, copies headers, and adds `X-Forwarded-For` when a peer
address is available. Request and response bodies are buffered, not streamed.

## Prerequisites

A reachable HTTP upstream server is required. The example below assumes one
is already listening on `127.0.0.1:3000`.

## Run the example

```sh
cd extras/http-proxy
cargo run -- 127.0.0.1 8080 127.0.0.1 3000
```

With these arguments, the proxy listens on `127.0.0.1:8080` and forwards to
`http://127.0.0.1:3000`. The four required positional arguments, in order,
are `listen_addr`, `listen_port`, `forward_addr`, and `forward_port`. Keep
Cargo's `--` separator before them.

## Try it

```sh
curl -i 'http://127.0.0.1:8080/example?name=ntex'
```

This forwards the request to `http://127.0.0.1:3000/example?name=ntex` and
returns the upstream status and body. The upstream decides the response.

## Tests

```sh
cargo test -p http-proxy
```

Covers request and response forwarding and upstream error responses using local test servers.

## Learn more

- [Handlers](https://docs.ntex.rs/8.1-web-app.html#handlers) and [application state](https://docs.ntex.rs/8.2-web-app.html)
- [HTTP clients](https://docs.ntex.rs/10-tls.html#http-client), including HTTPS upstreams
- [Test servers](https://docs.ntex.rs/5-server.html#test-servers)
