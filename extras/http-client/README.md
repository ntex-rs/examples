# Downloading an image over HTTPS

This example builds an `ntex::client::Client` with an OpenSSL
`SslConnector` and shares it with the download URL through `web::AppState`. See `src/main.rs` for
`Client::builder().secure_connector(...)` and the outgoing HTTPS request.

Each request to `/` downloads an approximately 10 MB Wikipedia image, buffers
it in memory with a 20,000,000-byte limit, and returns it as `image/jpeg`.
It does not stream the upstream body to the caller.

## Prerequisites

OpenSSL development libraries and internet access to
`upload.wikimedia.org` are required. `Cargo.toml` enables ntex's `openssl`
feature.

## Run the example

```sh
cd extras/http-client
cargo run
```

The server listens on `0.0.0.0:3000`, accepting connections on all interfaces.

## Try it

```sh
curl http://127.0.0.1:3000/ -o image.jpg
```

The handler prints the time spent downloading the image into memory.

## Tests

```sh
cargo test -p http-client
```

Covers downloading a binary image from a local test server and unknown routes without contacting Wikipedia.

## Learn more

- [HTTPS clients](https://docs.ntex.rs/10-tls.html#http-client)
- [Application State](https://docs.ntex.rs/8.2-web-app.html#accessing-application-state-in-handlers)
