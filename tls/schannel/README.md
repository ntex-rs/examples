# Windows Schannel: native TLS

Schannel is the TLS implementation built into Windows. It uses Windows
certificate stores and trust policy, and can use private keys that cannot be
exported. This example shows how to:

- serve HTTP over Schannel by composing the TLS acceptor with `HttpService`;
- call it from an ntex HTTP client that uses Windows' server verification;
- authenticate client certificates, which Schannel requests but does not
  enforce.

| Binary | Source | What it does |
| --- | --- | --- |
| `schannel-server` | [`src/main.rs`](src/main.rs) | HTTPS server; add `--mtls` to require the authorized client |
| `schannel-client` | [`src/client.rs`](src/client.rs) | HTTPS client, optionally presenting a client certificate |

## Before you start

- You need Windows. On other platforms, the binaries build but exit
  immediately with `Schannel requires Windows`.
- No Cargo feature is needed for Schannel; this package enables only the Tokio
  runtime.
- Create the [local certificates](../README.md#step-1-create-local-certificates).
  The script needs OpenSSL, which comes with Git Bash.
- Run every command below from this directory, `tls\schannel`.

## 1. Start the server

```powershell
cargo run --bin schannel-server
```

The server loads `../certs/server.pem` and `server.key`, converts the identity
to its native form with `Identity::schannel`, and puts `TlsAcceptor` in front of
`HttpService`.

There is no `bind_schannel` helper like `bind_openssl` or `bind_rustls`. This
service chain is how you use Schannel with a web app.

### Using a certificate you already have

Instead of the generated files, set `TLS_IDENTITY` before starting the server.
You can use a certificate-store selector:

```powershell
$env:TLS_IDENTITY = 'cert:CurrentUser\MY\subject=localhost'
```

or the path to a PFX file with an empty password. The selected certificate
needs an accessible private key, and its SANs and Extended Key Usage (EKU) must
allow it to act as a server for `localhost`.

There are two ways to select from a store:

| Selector | Picks |
| --- | --- |
| `cert:CurrentUser\MY\<40-hex SHA-1 thumbprint>` | Exactly that certificate. Use this when you need a predictable choice. |
| `cert:CurrentUser\MY\subject=<name>` | The currently valid match with the latest expiry |

Choosing a certificate does not make clients trust it.

The client reads `TLS_IDENTITY` too, so clear it before going on:

```powershell
Remove-Item Env:TLS_IDENTITY
```

## 2. Connect a client

`ClientConfig::new` verifies the server's certificate and hostname using
Windows trust. It has no option to add a CA of your own. For the local CA to be
trusted, Windows has to trust it.

Do this **only in a throwaway or isolated development account**:

```powershell
certutil -user -addstore Root ..\certs\ca.pem
cargo run --bin schannel-client -- https://localhost:8443/
```

The response is `hello over TLS`.

When you are done, remove the CA again. First, find its SHA-1 thumbprint:

```powershell
certutil -dump ..\certs\ca.pem
```

Then delete it from the store:

```powershell
certutil -user -delstore Root "<CA SHA-1 thumbprint>"
```

For a real server whose certificate Windows already trusts, you can skip these
steps. Either way, do not turn off verification to make an untrusted CA work.

## 3. Requiring client certificates

Restart the server with `--mtls`:

```powershell
cargo run --bin schannel-server -- --mtls
```

Connect with the authorized client identity. The last argument is a file stem:
it loads `client.pem` and `client.key`:

```powershell
cargo run --bin schannel-client -- https://localhost:8443/ ../certs/client
```

The response is `authorized`.

Schannel never picks a client certificate from your stores automatically; the
client sends one only because the program passes it explicitly. To use a store
certificate instead, set `TLS_IDENTITY` for the client. It takes precedence over
the file-stem argument.

### Why the application has to check

With `request_client_cert(true)`, Schannel asks the client for a certificate
and proof that it owns the key. That is all it does:

- the client may decline, and the handshake still succeeds;
- Schannel does not check whether the certificate is trusted.

So the server checks the client itself, before HTTP starts, using the shared
`authenticate` helper. That helper:

1. rejects connections without a certificate;
2. rejects certificates that do not chain to `ca.pem`;
3. rejects any identity other than `spiffe://ntex.example/client`;
4. closes the connection when the verified certificate path expires.

### Try to break it

Run the client without an identity, with `../certs/untrusted-client`, and with
`../certs/other-client`. None of them may receive an HTTP response. Unlike
OpenSSL and rustls, the TLS handshake usually *succeeds* here. The server
rejects the connection right after it, in the application check.

## Notes on keys

When a PEM identity is imported for Schannel, Windows stores its key
temporarily. ntex deletes it when the last `Identity` that uses it is dropped.
If the process is killed abruptly, a temporary key can be left behind.

## Learn more

From the guide chapter [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html):

- [Configuring Windows Schannel](https://docs.ntex.rs/10-tls.html#windows-schannel)
- [PKCS#12 and Windows stores](https://docs.ntex.rs/10-tls.html#pkcs12-and-windows-stores)
- [Using an identity with each backend](https://docs.ntex.rs/10-tls.html#using-an-identity-with-each-backend)
- [Client-certificate authentication](https://docs.ntex.rs/10-tls.html#client-certificate-authentication)
- [Backend differences](https://docs.ntex.rs/10-tls.html#backend-differences) in client-certificate checks

## Tests

```sh
cargo test -p schannel-example
```

This compiles the platform-gated binaries; no Schannel round-trip tests are provided, since they require Windows certificate handling that cannot be validated on other platforms.
