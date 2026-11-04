//! gRPC over TLS with ntex-grpc and rustls.
//!
//! The `helloworld.Greeter` service comes from the `grpc-helloworld`
//! example. This crate adds TLS: the server runs `TlsAcceptor` before
//! `GrpcServer`, and the client gives the h2 pool a `TlsConnector`.

use std::{error::Error as StdError, io::BufReader, sync::Arc};

use grpc_helloworld::{grpc_server, helloworld::GreeterClient};
use ntex::io::Io;
use ntex::tls::{rustls::TlsAcceptor, rustls::TlsConnector, x509::Identity};
use ntex::{Service, SharedCfg};
use ntex_grpc::client::Client;
use ntex_h2::client as h2;
use rustls::{ClientConfig, RootCertStore, ServerConfig, sign::SingleCertAndKey};
use tls_x509_example::Error;

/// Address the example server listens on.
pub const ADDR: &str = "127.0.0.1:50056";

/// Address the example client connects to. The host name must match the
/// server certificate.
pub const CONNECT_ADDR: &str = "localhost:50056";

/// gRPC needs HTTP/2, so both sides offer only `h2` in ALPN.
const ALPN_H2: &[u8] = b"h2";

/// Configures the server certificate and HTTP/2 ALPN.
pub fn server_config(identity: &Identity) -> Result<Arc<ServerConfig>, Error> {
    let key = ntex::tls::rustls::certified_key(identity)?;
    let mut config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(SingleCertAndKey::from(key)));
    config.alpn_protocols = vec![ALPN_H2.to_vec()];
    Ok(Arc::new(config))
}

/// Configures a client that trusts the CA certificates in `ca_pem`.
pub fn client_config(ca_pem: &[u8]) -> Result<ClientConfig, Error> {
    let mut roots = RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut BufReader::new(ca_pem)) {
        roots.add(certificate?)?;
    }
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
    config.alpn_protocols = vec![ALPN_H2.to_vec()];
    Ok(config)
}

/// Accepts TLS on a connection, then serves gRPC over it.
///
/// `GrpcServer` serves any `Io`, so the TLS stream is passed on as is.
pub fn grpc_tls_service(
    config: Arc<ServerConfig>,
) -> impl Service<(), Io, Res = (), Error = Box<dyn StdError>> {
    ntex::service(TlsAcceptor::new(config))
        .map_err(Box::<dyn StdError>::from)
        .and_then(grpc_server())
}

/// Builds a `Greeter` client that connects to `addr` over TLS.
///
/// `secure(true)` makes the client send the `https` scheme.
pub fn connect(addr: impl Into<String>, config: ClientConfig) -> GreeterClient<Client> {
    let pool = h2::Client::builder(addr.into())
        .connector(TlsConnector::new(config))
        .secure(true)
        .build(SharedCfg::new("GRPC-TLS-CLIENT"));
    GreeterClient::new(Client::new(pool))
}

#[cfg(test)]
mod tests {
    use super::*;
    use grpc_helloworld::{connect as connect_plain, helloworld::HelloRequest};
    use ntex::tls::x509::Certificate;
    use ntex::{
        server::{TestServer, test_server},
        time::{Seconds, timeout},
    };
    use ntex_tls::x509::testgen::{self, San};

    /// Generates a CA and a `localhost` server identity it issued.
    fn identities() -> (Certificate, Identity) {
        let ca_key = testgen::key();
        let ca = testgen::cert("test CA", true, None, &ca_key, &San::default(), (-1, 1));
        let key = testgen::key();
        let san = San {
            dns: &["localhost"],
            ..San::default()
        };
        let cert = testgen::cert(
            "localhost",
            false,
            Some((&ca, &ca_key)),
            &key,
            &san,
            (-1, 1),
        );
        let identity = Identity::from_der(&[cert.der(), ca.der()], &key.pkcs8_der()).unwrap();
        (ca, identity)
    }

    /// Starts the TLS gRPC service and returns it with the PEM of its CA.
    fn start() -> (TestServer, String) {
        let (ca, identity) = identities();
        let config = server_config(&identity).unwrap();
        let srv = test_server(async move |_| grpc_tls_service(config.clone()));
        (srv, ca.to_pem())
    }

    fn hello(name: &str) -> HelloRequest {
        HelloRequest { name: name.into() }
    }

    /// STATEMENT: A client that trusts the server CA calls `Greeter` over TLS.
    ///
    /// HOW:
    /// - Generate a CA and a `localhost` identity in memory and serve
    ///   `grpc_tls_service()` with them.
    /// - Connect to `localhost` with `client_config()` for that CA and call
    ///   `say_hello`, then check the reply within a timeout.
    #[ntex::test]
    async fn trusted_client_calls_over_tls() {
        let (srv, ca_pem) = start();
        let client = connect(
            format!("localhost:{}", srv.addr().port()),
            client_config(ca_pem.as_bytes()).unwrap(),
        );
        timeout(Seconds(3), async {
            let res = client.say_hello(&hello("tls")).send().await.unwrap();
            assert_eq!(res.message, "Hello tls!");
        })
        .await
        .unwrap();
    }

    /// STATEMENT: A client rejects a server whose certificate comes from a CA it does not trust.
    ///
    /// HOW:
    /// - Serve `grpc_tls_service()` with a generated identity.
    /// - Connect with `client_config()` for another generated CA and check
    ///   the call fails within a timeout.
    #[ntex::test]
    async fn untrusted_server_certificate_is_rejected() {
        let (srv, _) = start();
        let (other_ca, _) = identities();
        let client = connect(
            format!("localhost:{}", srv.addr().port()),
            client_config(other_ca.to_pem().as_bytes()).unwrap(),
        );
        timeout(Seconds(3), async {
            assert!(client.say_hello(&hello("tls")).send().await.is_err());
        })
        .await
        .unwrap();
    }

    /// STATEMENT: The TLS server does not serve gRPC to a plaintext client.
    ///
    /// HOW:
    /// - Serve `grpc_tls_service()` with a generated identity.
    /// - Call it with the plaintext client from `grpc-helloworld` and check
    ///   the call fails within a timeout.
    #[ntex::test]
    async fn plaintext_client_is_rejected() {
        let (srv, _) = start();
        let client = connect_plain(srv.addr().to_string());
        timeout(Seconds(3), async {
            assert!(client.say_hello(&hello("tls")).send().await.is_err());
        })
        .await
        .unwrap();
    }
}
