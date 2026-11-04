# Portable X.509 identities and client authentication

Generate the [local certificates](../README.md), then run from this directory:

```sh
cargo run -- pem ../certs/client.pem
cargo run -- der ../certs/client.der
cargo run -- identity ../certs/client
```

The output includes subject, issuer, SANs, validity and a SHA-256 thumbprint.
`identity` also loads the matching unencrypted PKCS#8 key, without printing it.
Parsing or loading an identity is not a trust check.

The default `rustls` feature enables the ring fallback for X.509. On Unix,
PKCS#12 additionally needs OpenSSL:

```sh
cargo run --features openssl -- pkcs12 ../certs/client.p12
```

The generated archive has an empty password, for local demonstration only.
For another archive, supply its password through `PFX_PASSWORD`, not a command
line argument. On Windows PKCS#12 is native; `--no-default-features` avoids
unnecessary rustls dependencies.

`source` uses `IdentitySource` syntax. A plain path means an empty-password
PKCS#12 archive, not a PEM file:

```sh
cargo run --features openssl -- source ../certs/client.p12
```

On Windows, an existing certificate and accessible private key can be selected
without exporting them:

```powershell
cargo run --no-default-features -- source 'cert:CurrentUser\MY\subject=client'
```

Store selection is not trust verification. A subject selector chooses a
currently valid match with the latest expiry; a thumbprint selector does not
filter by validity.

## Shared authentication helpers

`src/lib.rs` is also used by the backend examples. It demonstrates:

- DER peer metadata from `IoRef`, with missing certificates handled explicitly.
- `PeerCerts` parsing and separation of the leaf from peer-provided intermediates.
- A configured `CaCerts` store that rejects incomplete imports.
- Chain verification followed by URI-based application authorization.
- Termination at the verified path's expiry, with timer cancellation on disconnect.

The application policy accepts only `spiffe://ntex.example/client`. It is an
exact, case-insensitive match through `auth::name_matches`, not a general SPIFFE
implementation. No revocation service is consulted.

Windows always uses Crypt32/CNG for X.509. Elsewhere rustls takes precedence if
both backends are enabled; verification failure does not trigger a retry using
OpenSSL. To exercise only the OpenSSL implementation:

```sh
cargo run --no-default-features --features openssl -- pem ../certs/client.pem
```
