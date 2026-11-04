#![recursion_limit = "256"]

#[cfg(windows)]
#[ntex::main]
async fn main() -> Result<(), tls_x509_example::Error> {
    use ntex::io::{Io, Layer};
    use ntex::tls::schannel::{SchannelFilter, ServerConfig, TlsAcceptor};
    use ntex::tls::x509::IdentitySource;
    use ntex::web::{self, App, HttpResponse};
    use ntex::{Service, SharedCfg, http, service::fn_service};
    use tls_x509_example::{Error, authenticate, close_at_expiry, load_identity, load_trust};

    let mut args = std::env::args().skip(1);
    let mtls = args.next().as_deref() == Some("--mtls");
    let identity = if let Ok(source) = std::env::var("TLS_IDENTITY") {
        source.parse::<IdentitySource>()?.load()?
    } else {
        load_identity("../certs/server")?
    };
    let config = ServerConfig::new(identity.schannel().clone())?
        .set_alpn_protocols(&[b"http/1.1"])
        .request_client_cert(mtls);
    ntex::server::build()
        .bind(
            "TLS",
            "127.0.0.1:8443",
            SharedCfg::new("SCHANNEL"),
            async move |_| {
                let trust = load_trust("../certs/ca.pem").expect("client trust anchors");
                ntex::service(TlsAcceptor::new(config.clone()))
                    .map_err(Error::from)
                    .and_then(fn_service(async move |io: Io<Layer<SchannelFilter>>| {
                        if mtls {
                            // Schannel's certificate request alone is neither mandatory nor a trust check.
                            let expires = authenticate(&io, &trust)?;
                            close_at_expiry(&io, expires);
                            println!("Authorized client; verified path expires at {expires:?}");
                        }
                        Ok::<_, Error>(io)
                    }))
                    .and_then(
                        http::HttpService::new(
                            App::new().service(
                                web::resource("/")
                                    .to(|| async { HttpResponse::Ok().body("hello over TLS") }),
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

#[cfg(not(windows))]
fn main() -> Result<(), &'static str> {
    Err("Schannel requires Windows")
}
