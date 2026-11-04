# Windows Schannel

Schannel is available on Windows without an OpenSSL or rustls feature. This
package enables the Tokio runtime. Non-Windows binaries report that Windows is
required.

Generate the [local identities](../README.md) with OpenSSL (for example in Git
Bash). On Windows, imported `Identity` keys are retained for Schannel and
normally removed when their final owner drops; abrupt process termination can
leave temporary keys behind.

## Server

Run from this directory:

```powershell
cargo run --bin schannel-server
```

The server loads `../certs/server.pem` and `server.key`, obtains a native
identity with `Identity::schannel`, and composes `TlsAcceptor` with `HttpService`.
There is no `web::HttpServer::bind_schannel` helper.

To select an existing identity instead, set `TLS_IDENTITY` to an empty-password
PFX path or a store selector before running the server:

```powershell
$env:TLS_IDENTITY = 'cert:CurrentUser\MY\subject=localhost'
```

The selected certificate must have an accessible private key and appropriate
server SANs/EKU. Use a SHA-1 store thumbprint selector for deterministic selection.
Subject selection chooses a currently valid matching identity with the latest
expiry; selection itself does not verify trust. Remove the environment variable
before using the generated identities again.

## Client

`ClientConfig::new` uses Windows server-certificate and hostname verification.
There is no custom root-store setter on this configuration. For the local demo,
trust the generated CA **only in an isolated development Windows account**:

```powershell
certutil -user -addstore Root ..\certs\ca.pem
cargo run --bin schannel-client -- https://localhost:8443/
```

Expected response: `hello over TLS`. Remove the development root afterwards,
using the exact thumbprint printed by `certutil -dump ..\certs\ca.pem`:

```powershell
certutil -user -delstore Root "<CA SHA-1 thumbprint>"
```

Do not disable verification to work around an untrusted CA. For a real endpoint
with an already trusted certificate, no trust-store modification is necessary.

## Client-certificate authentication

Stop the ordinary server and run:

```powershell
cargo run --bin schannel-server -- --mtls
```

In another terminal:

```powershell
cargo run --bin schannel-client -- https://localhost:8443/ ../certs/client
```

The client supplies an explicit identity; Schannel does not automatically select
one from Windows stores. `TLS_IDENTITY` can select an existing client identity
instead of the PEM file stem.

`request_client_cert(true)` only requests proof of key possession. A handshake
can finish without a certificate, and Schannel does not validate client trust
here. The subsequent X.509 gate rejects missing certificates, untrusted chains,
and identities other than `spiffe://ntex.example/client` **before HTTP starts**.
It also bounds the connection lifetime by the verified path's expiry.

Repeat the client command without an identity, with `../certs/untrusted-client`,
and with `../certs/other-client`: none may receive an HTTP response. Unlike the
other backends, rejection may occur after a successful TLS handshake.
