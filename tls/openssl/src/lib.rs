//! TLS configuration shared by the HTTPS server, client, and mutual-TLS server.

use ntex::io::{Io, Layer};
use ntex::tls::{
    openssl::{SslAcceptor as Acceptor, SslFilter},
    x509::{CaCerts, Identity},
};
use ntex::web::{self, App, HttpResponse};
use ntex::{http, service::fn_service, Service};
use openssl::{
    ssl::{self, SslAcceptor, SslAcceptorBuilder, SslConnector, SslMethod, SslVerifyMode},
    x509::X509,
};
use tls_x509_example::{authenticate, close_at_expiry, Error};

/// Configures the HTTPS server with its certificate chain and key.
pub fn server_config(identity: &Identity) -> Result<SslAcceptorBuilder, Error> {
    let mut config = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
    ntex::tls::openssl::set_identity(&mut config, identity)?;
    Ok(config)
}

/// Configures server verification and an optional client identity.
pub fn client_config(ca_pem: &[u8], identity: Option<&Identity>) -> Result<SslConnector, Error> {
    let mut config = SslConnector::builder(SslMethod::tls())?;
    config.set_verify(SslVerifyMode::PEER);
    for certificate in X509::stack_from_pem(ca_pem)? {
        config.cert_store_mut().add_cert(certificate)?;
    }
    config.set_alpn_protos(b"\x02h2\x08http/1.1")?;
    if let Some(identity) = identity {
        ntex::tls::openssl::set_identity(&mut config, identity)?;
    }
    Ok(config.build())
}

/// Requires a certificate issued by the configured client CA during the handshake.
pub fn mtls_config(identity: &Identity, ca_pem: &[u8]) -> Result<SslAcceptor, Error> {
    let mut config = server_config(identity)?;
    for certificate in X509::stack_from_pem(ca_pem)? {
        config.cert_store_mut().add_cert(certificate)?;
    }
    config.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
    config.set_alpn_select_callback(|_, offered| {
        ssl::select_next_proto(b"\x08http/1.1", offered).ok_or(ssl::AlpnError::NOACK)
    });
    Ok(config.build())
}

/// Authenticates the client before exposing the HTTP application.
pub fn mtls_service(
    config: SslAcceptor,
    trust: CaCerts,
) -> impl Service<(), Io, Res = (), Error = Error> {
    ntex::service(Acceptor::new(config))
        .map_err(Error::from)
        .and_then(fn_service(async move |io: Io<Layer<SslFilter>>| {
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

// On Windows, `Identity` keys are non-exportable CNG keys, which OpenSSL cannot use.
#[cfg(all(test, not(windows)))]
mod test_support;

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use ntex::{
        client::Client,
        http::StatusCode,
        server::test_server,
        time::{timeout, Seconds},
        SharedCfg,
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
                .openssl(client_config(ca.to_pem().as_bytes(), None).unwrap())
                .build(SharedCfg::new("ANONYMOUS"));
            assert!(anonymous.get(url.as_str()).send().await.is_err());
            let authorized = Client::builder()
                .openssl(client_config(ca.to_pem().as_bytes(), Some(&client_identity)).unwrap())
                .build(SharedCfg::new("AUTHORIZED"));
            let response = authorized.get(url.as_str()).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.body().await.unwrap(), "authorized");
        })
        .await
        .unwrap();
    }
}
