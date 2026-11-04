# OpenSSL HTTPS and mutual authentication

Requires OpenSSL development libraries. The package enables ntex's `openssl`
and `tokio` features. Generate the [local certificates](../README.md) first.

## Server and client

From this directory, in separate terminals:

```sh
cargo run --bin openssl-example
cargo run --bin openssl-client -- https://localhost:8443/index.html
```

The server loads an `x509::Identity`, installs it with `set_identity`, and uses
`bind_openssl`. The client configures peer verification, the local CA, and HTTP
ALPN before passing its configuration to `ClientBuilder::openssl`.

An equivalent request with curl:

```sh
curl --cacert ../certs/ca.pem https://localhost:8443/index.html
```

The expected response is `Welcome!`. No `-k` option is needed.

## Mandatory client authentication

Stop the ordinary server, then run:

```sh
cargo run --bin openssl-mtls-server
```

In another terminal:

```sh
cargo run --bin openssl-client -- https://localhost:8443/ ../certs/client
```

The expected response is `authorized`. The client loads `client.pem` and
`client.key`. The server uses `PEER | FAIL_IF_NO_PEER_CERT`, then applies the
portable X.509 trust and URI policy before handing I/O to `HttpService`.

These requests must fail without reaching the HTTP handler:

```sh
cargo run --bin openssl-client -- https://localhost:8443/
cargo run --bin openssl-client -- https://localhost:8443/ ../certs/untrusted-client
cargo run --bin openssl-client -- https://localhost:8443/ ../certs/other-client
```

The first two fail TLS authentication; the last passes TLS but fails application
authorization. The server closes the connection rather than returning an HTTP
error because the gate runs before HTTP.

`mozilla_intermediate_v5` allows modern TLS, including TLS 1.3. The lower-level
mutual-authentication example advertises only `http/1.1`; the ordinary web-server
helper configures HTTP ALPN itself.
