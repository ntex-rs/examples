# rustls HTTPS and mutual authentication

The package enables ntex's `rustls` and `tokio` features and configures rustls
with ring explicitly. The X.509 helper enables `ntex-tls/rustls-ring`, allowing
identity loading and trust verification to use ring without installing a
process-global provider. Generate the [local certificates](../README.md) first.

## Server and client

From this directory, in separate terminals:

```sh
cargo run --bin rustls-server
cargo run --bin rustls-client -- https://localhost:8443/index.html
```

The server loads an `x509::Identity`, converts it with `certified_key`, and
installs a `SingleCertAndKey` resolver. The client loads the CA into a
`RootCertStore`, enables server-name verification, and configures HTTP ALPN.

An equivalent request:

```sh
curl --cacert ../certs/ca.pem https://localhost:8443/index.html
```

The expected response is `Welcome!`.

## Mandatory client authentication

Stop the ordinary server, then run:

```sh
cargo run --bin rustls-mtls-server
```

In another terminal:

```sh
cargo run --bin rustls-client -- https://localhost:8443/ ../certs/client
```

The expected response is `authorized`. The client uses its own certificate
resolver. The server's `WebPkiClientVerifier` requires a trusted certificate;
the portable X.509 gate additionally authorizes its URI before HTTP starts.

These requests must fail without reaching the HTTP handler:

```sh
cargo run --bin rustls-client -- https://localhost:8443/
cargo run --bin rustls-client -- https://localhost:8443/ ../certs/untrusted-client
cargo run --bin rustls-client -- https://localhost:8443/ ../certs/other-client
```

The first two fail TLS authentication; the last fails application authorization
after TLS. Both examples support TLS 1.2 and TLS 1.3. The mutual-authentication
server advertises only `http/1.1`.
