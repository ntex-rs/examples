# Chaining asynchronous HTTP requests

This example extracts JSON with `web::types::Json`, validates it with the
`validator` crate, and passes it through three asynchronous HTTP requests.
Each call to `step_x` in `src/main.rs` posts the data to httpbin and reads the
echoed JSON before the handler returns the final value. Application state holds
the HTTP client and upstream URL, which defaults to `https://httpbin.org/post`.

## Prerequisites

Internet access to `https://httpbin.org/post` and OpenSSL development libraries
are required. The run command enables ntex's OpenSSL feature because this
crate does not enable a TLS backend in its manifest.

## Run the example

```sh
cd extras/async-ex1
cargo run --features ntex/openssl
```

The server listens on `127.0.0.1:8080`.

## Try it

```sh
curl -X POST http://127.0.0.1:8080/something \
  -H 'Content-Type: application/json' \
  -d '{"id":"1","name":"JohnDoe"}'
```

Or, with [HTTPie](https://httpie.io/):

```sh
http POST http://127.0.0.1:8080/something id=1 name=JohnDoe
```

`id` must contain 1–1,000,000 characters and `name` 1–100 characters.
On success, the response contains the same `id` and `name`; validation failures
return a bad-request error. Each successful request makes three network calls.

## Tests

```sh
cargo test -p async-ex1
```

Covers three chained requests against a local test upstream, validation failures, and malformed JSON without internet access.

## Learn more

- [Extractors and Responders](https://docs.ntex.rs/8.4-web-app.html#request-bodies)
- [HTTPS clients](https://docs.ntex.rs/10-tls.html#http-client)
