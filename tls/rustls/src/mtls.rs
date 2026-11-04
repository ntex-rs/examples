#![recursion_limit = "256"]

use std::{fs::File, io::BufReader, sync::Arc};

use ntex::io::{Io, Layer};
use ntex::tls::rustls::{TlsAcceptor, TlsServerFilter};
use ntex::web::{self, App, HttpResponse};
use ntex::{Service, SharedCfg, http, service::fn_service};
use rustls::{RootCertStore, ServerConfig, server::WebPkiClientVerifier, sign::SingleCertAndKey};
use tls_x509_example::{Error, authenticate, close_at_expiry, load_identity, load_trust};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut BufReader::new(File::open("../certs/ca.pem")?)) {
        roots.add(cert?)?;
    }
    let verifier =
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone()).build()?;
    let key = ntex::tls::rustls::certified_key(&load_identity("../certs/server")?)?;
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_client_cert_verifier(verifier)
        .with_cert_resolver(Arc::new(SingleCertAndKey::from(key)));
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let config = Arc::new(config);
    ntex::server::build()
        .bind(
            "MTLS",
            "127.0.0.1:8443",
            SharedCfg::new("MTLS"),
            async move |_| {
                let trust = load_trust("../certs/ca.pem").expect("client trust anchors");
                ntex::service(TlsAcceptor::new(config.clone()))
                    .map_err(Error::from)
                    .and_then(fn_service(async move |io: Io<Layer<TlsServerFilter>>| {
                        let expires = authenticate(&io, &trust)?;
                        close_at_expiry(&io, expires);
                        println!("Authorized client; verified path expires at {expires:?}");
                        Ok::<_, Error>(io)
                    }))
                    .and_then(
                        http::HttpService::new(
                            App::new().service(
                                web::resource("/")
                                    .to(|| async { HttpResponse::Ok().body("authorized") }),
                            ),
                        )
                        .map_err(Error::from),
                    )
            },
        )?
        .run()
        .await?;
    Ok(())
}
