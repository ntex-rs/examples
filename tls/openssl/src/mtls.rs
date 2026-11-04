#![recursion_limit = "256"]

use ntex::io::{Io, Layer};
use ntex::tls::openssl::{SslAcceptor, SslFilter};
use ntex::web::{self, App, HttpResponse};
use ntex::{http, service::fn_service, Service, SharedCfg};
use openssl::ssl::{self, SslMethod, SslVerifyMode};
use tls_x509_example::{authenticate, close_at_expiry, load_identity, load_trust, Error};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();
    let mut config = ssl::SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
    ntex::tls::openssl::set_identity(&mut config, &load_identity("../certs/server")?)?;
    config.set_ca_file("../certs/ca.pem")?;
    config.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
    config.set_alpn_select_callback(|_, offered| {
        ssl::select_next_proto(b"\x08http/1.1", offered).ok_or(ssl::AlpnError::NOACK)
    });
    let config = config.build();
    ntex::server::build()
        .bind(
            "MTLS",
            "127.0.0.1:8443",
            SharedCfg::new("MTLS"),
            async move |_| {
                let trust = load_trust("../certs/ca.pem").expect("client trust anchors");
                ntex::service(SslAcceptor::new(config.clone()))
                    .map_err(Error::from)
                    .and_then(fn_service(async move |io: Io<Layer<SslFilter>>| {
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
