# OpenSSL: HTTPS and mutual TLS

This example shows how to:

- serve HTTPS with OpenSSL;
- call it from an ntex HTTP client that verifies the server;
- require and check client certificates (mutual TLS).

| Binary | Source | What it does |
| --- | --- | --- |
| `openssl-example` | [`src/main.rs`](src/main.rs) | HTTPS web server using `bind_openssl` |
| `openssl-client` | [`src/client.rs`](src/client.rs) | HTTPS client, optionally presenting a client certificate |
| `openssl-mtls-server` | [`src/mtls.rs`](src/mtls.rs) | Server that accepts only the authorized client |

## Before you start

- Install the OpenSSL development libraries and headers.
- Create the [local certificates](../README.md#step-1-create-local-certificates).
- Run every command below from this directory, `tls/openssl`.
- This example does not run on Windows. There, `x509::Identity` keeps its
  private key as a non-exportable CNG key, which OpenSSL cannot load. Use the
  [rustls](../rustls) or [schannel](../schannel) example on Windows.

## 1. HTTPS server and client

Start the server in one terminal:

```sh
cargo run --bin openssl-example
```

Then send a request from another:

```sh
cargo run --bin openssl-client -- https://localhost:8443/index.html
```

You should see `Status: 200 OK` followed by `Welcome!`. curl works too, and
because it is told to trust the local CA, it needs no `-k`:

```sh
curl --cacert ../certs/ca.pem https://localhost:8443/index.html
```

### What to look for in the code

- **Server:** loads the identity with the shared `load_identity` helper,
  installs it with `ntex::tls::openssl::set_identity`, and serves the app with
  `bind_openssl`. The `mozilla_intermediate_v5` preset enables modern TLS,
  including TLS 1.3. `bind_openssl` configures HTTP ALPN.
- **Client:** turns on peer verification (`SslVerifyMode::PEER`), trusts
  `../certs/ca.pem`, offers `h2` and `http/1.1` through ALPN, and passes the
  configuration to `ClientBuilder::openssl`. ntex checks that the certificate
  matches the hostname in the URL.

## 2. Requiring client certificates

Stop the first server and start the mutual-TLS server:

```sh
cargo run --bin openssl-mtls-server
```

Connect with the authorized client identity. The last argument is a file stem:
it loads `client.pem` and `client.key`:

```sh
cargo run --bin openssl-client -- https://localhost:8443/ ../certs/client
```

The response is `authorized`.

### Try to break it

Each of these must be rejected before reaching HTTP:

```sh
# No client certificate
cargo run --bin openssl-client -- https://localhost:8443/

# Correct identity, untrusted issuer
cargo run --bin openssl-client -- https://localhost:8443/ ../certs/untrusted-client

# Trusted issuer, wrong identity
cargo run --bin openssl-client -- https://localhost:8443/ ../certs/other-client
```

The first two fail during the TLS handshake. `other-client` completes the
handshake because its certificate is valid, but the server's application check
rejects its URI and closes the connection without sending an HTTP response.

### What to look for in the code

- `PEER | FAIL_IF_NO_PEER_CERT` makes the client certificate mandatory.
  `PEER` alone would only request one.
- `set_ca_file` sets which CAs the server trusts for client certificates.
- After the handshake, the shared `authenticate` helper checks trust and the
  URI, and `close_at_expiry` closes the connection when the certificate path
  expires.
- This server builds its service chain by hand rather than using
  `bind_openssl`, so it sets ALPN to `http/1.1` itself.

## Learn more

From the guide chapter [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html):

- [Configuring OpenSSL](https://docs.ntex.rs/10-tls.html#openssl)
- [Configuring ntex components](https://docs.ntex.rs/10-tls.html#configuring-ntex-components): the HTTP
  client, WebSocket client, and servers
- [Client-certificate authentication](https://docs.ntex.rs/10-tls.html#client-certificate-authentication)
- [Reading peer certificates](https://docs.ntex.rs/10-tls.html#reading-peer-certificates)
- [Trust and application authorization](https://docs.ntex.rs/10-tls.html#trust-and-application-authorization)

## Tests

```sh
cargo test -p openssl-example
```

The tests generate identities in memory and exercise HTTPS routes and the real mutual-TLS service with anonymous and authorized clients.

The tests are skipped on Windows for the reason given in [Before you start](#before-you-start).
