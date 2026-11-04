# TLS and X.509 examples

These programs accompany the guide chapter
[Securing Connections with TLS](https://docs.ntex.rs/10-tls.html). Each one is
small enough to read in a few minutes, and every connection verifies certificates
properly. Nothing here works by switching verification off.

## Pick an example

| Directory | What you will learn |
| --- | --- |
| [openssl](openssl) | An HTTPS server and client with OpenSSL, plus a server that requires client certificates |
| [rustls](rustls) | The same programs with rustls, including how to choose a crypto provider |
| [schannel](schannel) | Native Windows TLS, certificate stores, and enforcing client authentication in the application |
| [x509](x509) | Inspecting certificates, loading identities, and the authentication helpers shared by the other examples |
| [../websockets/basic](../websockets/basic) | One set of WebSocket handlers serving both `ws://` and `wss://` |
| [../state/state-accumulation](../state/state-accumulation) | Running the TLS handshake yourself before building per-connection state |
| [../extras/http-client](../extras/http-client) | Plugging a custom HTTPS connector into the HTTP client |

New to TLS in ntex? Start with [rustls](rustls) or [openssl](openssl): run the
server and client, then try the mutual-authentication server.

## Step 1: create local certificates

The examples need a certificate authority (CA) and a few certificates signed by
it. A script creates them for you. From the repository root, with OpenSSL 3
installed, run:

```sh
sh tls/generate-certs.sh
```

This creates `tls/certs/`:

| File | Purpose |
| --- | --- |
| `ca.pem` | The local CA. Clients trust it to verify the server, and mTLS servers trust it to verify clients. |
| `server.pem`, `server.key` | Server identity for `localhost`, `127.0.0.1`, and `::1` |
| `client.pem`, `client.key` | The authorized client, `spiffe://ntex.example/client` |
| `other-client.pem`, `other-client.key` | Signed by the trusted CA, but with the wrong identity, `spiffe://ntex.example/other` |
| `untrusted-client.pem`, `untrusted-client.key` | The right identity, signed by a CA the server does not trust |
| `untrusted-ca.pem` | The CA that signed `untrusted-client`. No example trusts it. |
| `client.der`, `client.p12` | The authorized client in DER and PKCS#12 form, used by the [x509](x509) inspector |

A few things to know about these files:

- The certificates are valid for **seven days**. When they expire, move
  `tls/certs` aside and run the script again. It never overwrites an existing
  directory.
- Private keys are unencrypted PKCS#8 files. `tls/certs/` is gitignored; never
  commit or deploy these keys.
- Clients trust `ca.pem` explicitly, so you do not need `curl -k` or similar
  shortcuts. Do not add this CA to a production trust store. Schannel is the
  exception, because it uses Windows trust; its README explains a safe way to
  run it.

## Step 2: run an example

- All servers listen on `127.0.0.1:8443`, so run one at a time.
- Run each program from its own directory, for example `cd tls/rustls`, so the
  relative paths to `../certs` resolve.

## How the mutual-authentication servers work

The `*-mtls-server` programs, and Schannel with `--mtls`, put an
authentication step between TLS and HTTP:

```text
TLS acceptor -> client certificate required? trusted? allowed? -> HTTP service
```

These are three separate checks, and the three test clients show the difference:

| Client | Has a certificate | Trusted issuer | Allowed identity | Result |
| --- | --- | --- | --- | --- |
| *(none)* | no | — | — | rejected |
| `untrusted-client` | yes | no | yes | rejected |
| `other-client` | yes | yes | no | rejected |
| `client` | yes | yes | yes | `authorized` |

Rejected clients never reach an HTTP handler. The server closes the connection
instead of sending an error response.

The backends differ in where a rejection happens:

- **OpenSSL and rustls** require a trusted client certificate during the TLS
  handshake, so the first two clients fail there.
- **Schannel** only *asks* for a certificate and does not check its trust. The
  handshake can succeed, so the application step must do the rejecting.

The shared helpers in [`x509/src/lib.rs`](x509/src/lib.rs) implement that step
the same way for every backend:

1. Read the peer certificate from the connection.
2. Verify its chain against the configured CA.
3. Check its URI.
4. Close the connection when the verified certificate path expires.

These examples do not cover revocation checking or reloading the policy while
the server runs.

## Error handling

The examples return `Box<dyn std::error::Error>`. That keeps the original
handshake, certificate, and HTTP errors intact. Avoid flattening them into an
`io::Error` that contains only a string.

## Learn more

The guide chapter [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html) explains the ideas behind
these examples:

- [Quick start: an HTTPS server](https://docs.ntex.rs/10-tls.html#quick-start-an-https-server)
- [Choosing a backend](https://docs.ntex.rs/10-tls.html#choosing-a-backend)
- [TLS as an I/O filter](https://docs.ntex.rs/10-tls.html#tls-as-an-io-filter): how TLS fits under any
  ntex service
- [Certificates in a nutshell](https://docs.ntex.rs/10-tls.html#certificates-in-a-nutshell)
- [Client-certificate authentication](https://docs.ntex.rs/10-tls.html#client-certificate-authentication)
  and [trust and application authorization](https://docs.ntex.rs/10-tls.html#trust-and-application-authorization)
- [Troubleshooting](https://docs.ntex.rs/10-tls.html#troubleshooting) and the
  [production checklist](https://docs.ntex.rs/10-tls.html#production-checklist)
