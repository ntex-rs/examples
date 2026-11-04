//! TLS configuration shared by the HTTPS server, client, and mutual-TLS server.

use std::{io::BufReader, sync::Arc};

use ntex::io::{Io, Layer};
use ntex::tls::{
    rustls::{TlsAcceptor, TlsServerFilter},
    x509::{CaCerts, Identity},
};
use ntex::web::{self, App, HttpResponse};
use ntex::{Service, http, service::fn_service};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig, server::WebPkiClientVerifier, sign::SingleCertAndKey,
};
use tls_x509_example::{Error, authenticate, close_at_expiry};

fn roots(ca_pem: &[u8]) -> Result<RootCertStore, Error> {
    let mut roots = RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut BufReader::new(ca_pem)) {
        roots.add(certificate?)?;
    }
    Ok(roots)
}

/// Configures the HTTPS server with its certificate chain and key.
pub fn server_config(identity: &Identity) -> Result<ServerConfig, Error> {
    let key = ntex::tls::rustls::certified_key(identity)?;
    Ok(
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(SingleCertAndKey::from(key))),
    )
}

/// Configures server verification and an optional client identity.
pub fn client_config(ca_pem: &[u8], identity: Option<&Identity>) -> Result<ClientConfig, Error> {
    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots(ca_pem)?);
    let mut config = if let Some(identity) = identity {
        let key = ntex::tls::rustls::certified_key(identity)?;
        builder.with_client_cert_resolver(Arc::new(SingleCertAndKey::from(key)))
    } else {
        builder.with_no_client_auth()
    };
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

/// Requires a certificate issued by the configured client CA during the handshake.
pub fn mtls_config(identity: &Identity, ca_pem: &[u8]) -> Result<Arc<ServerConfig>, Error> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier =
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots(ca_pem)?), provider.clone())
            .build()?;
    let key = ntex::tls::rustls::certified_key(identity)?;
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_client_cert_verifier(verifier)
        .with_cert_resolver(Arc::new(SingleCertAndKey::from(key)));
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

/// Authenticates the client before exposing the HTTP application.
pub fn mtls_service(
    config: Arc<ServerConfig>,
    trust: CaCerts,
) -> impl Service<(), Io, Res = (), Error = Error> {
    ntex::service(TlsAcceptor::new(config))
        .map_err(Error::from)
        .and_then(fn_service(async move |io: Io<Layer<TlsServerFilter>>| {
            let expires = authenticate(&io, &trust)?;
            close_at_expiry(&io, expires);
            println!("Authorized client; verified path expires at {expires:?}");
            Ok::<_, Error>(io)
        }))
        .and_then(
            http::HttpService::new(App::new().service(
                web::resource("/").to(|| async { HttpResponse::Ok().body("authorized") }),
            ))
            .map_err(Error::from),
        )
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{
        SharedCfg,
        client::Client,
        http::StatusCode,
        server::test_server,
        time::{Seconds, timeout},
    };

    /// STATEMENT: The mutual-TLS service rejects anonymous clients and serves an authorized client.
    ///
    /// HOW:
    /// - Generate server, CA, and allowed client identities in memory and start the real mTLS service.
    /// - Use the example's client configuration without a certificate and check the request fails.
    /// - Repeat with the trusted URI-bearing client certificate and check `200` and `authorized`.
    #[ntex::test]
    async fn mtls_requires_an_authorized_client_certificate() {
        let (ca, identity, client_identity) = test_support::identities();
        let config = mtls_config(&identity, ca.to_pem().as_bytes()).unwrap();
        let trust = ca.clone();
        let srv = test_server(async move |_| {
            mtls_service(config.clone(), CaCerts::from_certs([trust.clone()]))
        });
        let url = format!("https://localhost:{}/", srv.addr().port());
        timeout(Seconds(3), async {
            let anonymous = Client::builder()
                .rustls(client_config(ca.to_pem().as_bytes(), None).unwrap())
                .build(SharedCfg::new("ANONYMOUS"));
            assert!(anonymous.get(url.as_str()).send().await.is_err());
            let authorized = Client::builder()
                .rustls(client_config(ca.to_pem().as_bytes(), Some(&client_identity)).unwrap())
                .build(SharedCfg::new("AUTHORIZED"));
            let response = authorized.get(url.as_str()).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.body().await.unwrap(), "authorized");
        })
        .await
        .unwrap();
    }
}
