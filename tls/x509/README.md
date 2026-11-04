# Portable X.509: inspecting certificates and authenticating clients

This crate has two parts:

- a small **certificate inspector** you can run against your own files;
- a **library of authentication helpers**, [`src/lib.rs`](src/lib.rs), used by
  the OpenSSL, rustls, and Schannel examples.

Both use `ntex::tls::x509`, which provides the same API on every platform and
backend.

## Before you start

Create the [local certificates](../README.md#step-1-create-local-certificates),
then run the commands below from this directory, `tls/x509`.

## Inspecting certificates

Choose a format and a file:

```sh
cargo run -- pem ../certs/client.pem
cargo run -- der ../certs/client.der
cargo run -- identity ../certs/client     # loads client.pem and client.key
```

Each command prints the subject, issuer, subject alternative names (SANs),
validity period, and SHA-256 thumbprint. The `identity` mode also loads the
matching private key and checks that it matches the certificate; it never
prints the key. Keys must be unencrypted PKCS#8.

Loading a certificate tells you what it *says*, not whether to *believe* it.
Trust is a separate check, covered below.

### PKCS#12 archives

On Linux and macOS, reading PKCS#12 (`.p12` or `.pfx`) requires the `openssl`
feature:

```sh
cargo run --features openssl -- pkcs12 ../certs/client.p12
```

The generated archive has an empty password. For a real archive, pass the
password through the `PFX_PASSWORD` environment variable, never as a
command-line argument. On Windows, PKCS#12 support is built in, so you can
build with `--no-default-features`.

### Identity sources

`source` accepts the same strings as `IdentitySource`. A plain path is treated
as an **empty-password PKCS#12 archive**, not as a PEM file:

```sh
cargo run --features openssl -- source ../certs/client.p12
```

On Windows, `source` can also select a certificate and its private key from a
store, without exporting the key:

```powershell
cargo run --no-default-features -- source 'cert:CurrentUser\MY\subject=client'
```

A `subject=` selector picks the currently valid match with the latest expiry. A
thumbprint selector picks exactly one certificate, even if it has expired.
Neither one checks trust.

### Which backend does the work?

| Platform | X.509 implementation |
| --- | --- |
| Windows | Always the native Crypt32/CNG APIs |
| Elsewhere, `rustls` feature (default) | rustls/WebPKI with ring |
| Elsewhere, `openssl` feature only | OpenSSL |

If both features are enabled outside Windows, rustls is used. A failed
verification is not retried with OpenSSL. To try the OpenSSL path on its own:

```sh
cargo run --no-default-features --features openssl -- pem ../certs/client.pem
```

## The authentication helpers

[`src/lib.rs`](src/lib.rs) shows a complete client-certificate policy that you
can adapt:

| Helper | What it does |
| --- | --- |
| `load_identity(stem)` | Loads `<stem>.pem` and `<stem>.key` into an `Identity` |
| `load_trust(path)` | Builds a `CaCerts` trust store and fails if any certificate is rejected, instead of continuing with fewer |
| `peer_certs(io)` | Reads the peer's certificate and chain from any TLS backend; returns `None` when there is no certificate |
| `authorize(peer, trust)` | Verifies the chain, then requires the URI `spiffe://ntex.example/client`; returns when the verified path expires |
| `authenticate(io, trust)` | Combines the two above and rejects connections without a certificate |
| `close_at_expiry(io, time)` | Closes the connection at the expiry time, and stops waiting if the connection closes first |

`AuthError` keeps the possible failures apart:

- `MissingCertificate`: the client sent no certificate;
- `Trust`: the chain is not trusted;
- `Name`: the identity is not allowed;
- `Certificate`: the certificate could not be parsed.

That makes it easy to log or count each kind separately.

A few limits to keep in mind:

- The URI check is an exact, case-insensitive match using
  `auth::name_matches`. It is not a full SPIFFE implementation.
- Revocation is not checked.
- The certificates in the client's chain are treated only as possible
  intermediates. Trust comes solely from `ca.pem`.

Run the helpers' tests with `cargo test`.

## Learn more

From the guide chapter [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html):

- [X.509 certificates and identity loading](https://docs.ntex.rs/10-tls.html#x509-certificates-and-identity-loading)
- [TLS and X.509 backend selection are different](https://docs.ntex.rs/10-tls.html#tls-and-x509-backend-selection-are-different)
- [Reading peer certificates](https://docs.ntex.rs/10-tls.html#reading-peer-certificates)
- [Trust and application authorization](https://docs.ntex.rs/10-tls.html#trust-and-application-authorization),
  including [expiry and long-lived connections](https://docs.ntex.rs/10-tls.html#expiry-and-long-lived-connections)

## Tests

```sh
cargo test -p tls-x509-example
```

The tests cover trusted client identities, URI authorization failures, missing certificates, and connection expiry using generated certificates and in-memory I/O.
