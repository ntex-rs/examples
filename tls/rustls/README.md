# rustls: HTTPS and mutual TLS

This example shows how to:

- serve HTTPS with rustls, choosing the crypto provider explicitly;
- call it from an ntex HTTP client that verifies the server;
- require and check client certificates (mutual TLS).

| Binary | Source | What it does |
| --- | --- | --- |
| `rustls-server` | [`src/main.rs`](src/main.rs) | HTTPS web server using `bind_rustls` |
| `rustls-client` | [`src/client.rs`](src/client.rs) | HTTPS client, optionally presenting a client certificate |
| `rustls-mtls-server` | [`src/mtls.rs`](src/mtls.rs) | Server that accepts only the authorized client |

## Before you start

- No system TLS library is needed. rustls and its
  [ring](https://github.com/briansmith/ring) crypto provider are pure Rust
  dependencies.
- Create the [local certificates](../README.md#step-1-create-local-certificates).
- Run every command below from this directory, `tls/rustls`.

### About crypto providers

rustls needs a *crypto provider*, the code that implements the cryptographic
algorithms. These programs pass ring to each builder with
`builder_with_provider`, so they never depend on a process-wide default.

The shared X.509 helpers enable `ntex-tls/rustls-ring`. That gives identity
loading and trust checks the same provider, again without installing a global
default.

## 1. HTTPS server and client

Start the server in one terminal:

```sh
cargo run --bin rustls-server
```

Then send a request from another:

```sh
cargo run --bin rustls-client -- https://localhost:8443/index.html
```

You should see `Status: 200 OK` followed by `Welcome!`. The equivalent curl
command is:

```sh
curl --cacert ../certs/ca.pem https://localhost:8443/index.html
```

### What to look for in the code

- **Server:** loads a portable `x509::Identity` and converts it to a rustls
  signing key with `certified_key`. A `SingleCertAndKey` resolver presents it
  during the handshake. Here, `with_no_client_auth` means "don't ask clients
  for a certificate." `bind_rustls` configures HTTP ALPN.
- **Client:** loads `ca.pem` into a `RootCertStore` and offers `h2` and
  `http/1.1`. Here, `with_no_client_auth` means "don't *send* a certificate."
  The client still verifies the server's certificate and hostname.

Both sides support TLS 1.2 and TLS 1.3.

## 2. Requiring client certificates

Stop the first server and start the mutual-TLS server:

```sh
cargo run --bin rustls-mtls-server
```

Connect with the authorized client identity. The last argument is a file stem:
it loads `client.pem` and `client.key`:

```sh
cargo run --bin rustls-client -- https://localhost:8443/ ../certs/client
```

The response is `authorized`.

### Try to break it

Each of these must be rejected before reaching HTTP:

```sh
# No client certificate
cargo run --bin rustls-client -- https://localhost:8443/

# Correct identity, untrusted issuer
cargo run --bin rustls-client -- https://localhost:8443/ ../certs/untrusted-client

# Trusted issuer, wrong identity
cargo run --bin rustls-client -- https://localhost:8443/ ../certs/other-client
```

The first two fail during the TLS handshake. `other-client` gets through TLS,
then the application check rejects its URI and closes the connection.

### What to look for in the code

- `WebPkiClientVerifier` with the CA as its only root makes a trusted client
  certificate mandatory. Do not enable anonymous clients if you rely on this.
- The client presents its identity through a `SingleCertAndKey` client
  resolver instead of `with_no_client_auth`.
- After the handshake, the shared `authenticate` helper checks trust and the
  URI, and `close_at_expiry` closes the connection when the certificate path
  expires.
- This server builds its service chain by hand rather than using `bind_rustls`,
  so it sets ALPN to `http/1.1` itself.

## Learn more

From the guide chapter [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html):

- [Configuring rustls](https://docs.ntex.rs/10-tls.html#rustls), including crypto providers
- [Configuring ntex components](https://docs.ntex.rs/10-tls.html#configuring-ntex-components): the HTTP
  client, WebSocket client, and servers
- [Client-certificate authentication](https://docs.ntex.rs/10-tls.html#client-certificate-authentication)
- [Reading peer certificates](https://docs.ntex.rs/10-tls.html#reading-peer-certificates)
- [Trust and application authorization](https://docs.ntex.rs/10-tls.html#trust-and-application-authorization)

## Tests

```sh
cargo test -p rustls-example
```

The tests generate identities in memory and exercise HTTPS routes and the real mutual-TLS service with anonymous and authorized clients.
