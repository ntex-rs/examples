# TLS and X.509 examples

These examples accompany the [TLS guide](https://docs.ntex.rs/10-tls.html).
They use verified local TLS connections, not certificate-verification bypasses.

| Directory | Demonstrates |
| --- | --- |
| [openssl](openssl) | HTTPS server/client, portable identities, mandatory client certificates |
| [rustls](rustls) | Explicit ring provider, trust roots, identities and mutual authentication |
| [schannel](schannel) | Windows TLS, certificate stores and application-side client authentication |
| [x509](x509) | Certificate inspection, identity loading and shared peer authentication helpers |
| [../websockets/basic](../websockets/basic) | The same WebSocket handlers over plain TCP or TLS |
| [../state/state-accumulation](../state/state-accumulation) | Explicit TLS handshake and connection-state composition |
| [../extras/http-client](../extras/http-client) | Custom HTTPS connector integration |

## Local certificates

From the examples repository root, with OpenSSL 3 available:

```sh
sh tls/generate-certs.sh
```

The script creates `tls/certs/` with unencrypted PKCS#8 keys and certificates
valid for seven days. It refuses to overwrite an existing directory. Move that
directory aside before regenerating expired certificates. The directory is
gitignored; never commit or deploy its private keys.

`ca.pem` signs the server certificate (SANs: `localhost`, `127.0.0.1`, `::1`)
and two client certificates. `client.pem` has the authorized URI
`spiffe://ntex.example/client`; `other-client.pem` is trusted but has a different
URI. `untrusted-client.pem` has the authorized URI but a different issuer.
These distinguish certificate trust from application authorization.

Clients load `ca.pem` explicitly. Schannel instead uses Windows server-trust
policy; see its README before running the local client. Do not disable server
verification or install this development CA in a production trust store.

## Authentication behavior

Start one server at a time: they use `127.0.0.1:8443`. Run each binary from its
own example directory so relative certificate paths resolve correctly.

The mutual-authentication servers compose:

```text
TLS acceptor -> required certificate + trust + URI policy -> HTTP service
```

Only the authorized client reaches HTTP. OpenSSL and rustls also require a
trusted client certificate during the handshake. Schannel's certificate request
is optional and does not check trust, so its application gate is essential.

The shared helpers in `x509/src/lib.rs` read backend-independent peer metadata,
verify the path, authorize the URI and terminate the connection when the verified
path expires. The expiry task stops when the connection disconnects. Revocation
checking and dynamic policy updates are outside these examples.

The example libraries use `Box<dyn std::error::Error>` to preserve errors,
including worker-local errors; they do not stringify errors into `io::Error`.
