//! Mutual-TLS server: requires a trusted client certificate during the handshake,
//! then allows only the `spiffe://ntex.example/client` identity to reach HTTP.

#![recursion_limit = "256"]

use ntex::SharedCfg;
use openssl_example::{mtls_config, mtls_service};
use tls_x509_example::{load_identity, load_trust, Error};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();
    let config = mtls_config(
        &load_identity("../certs/server")?,
        &std::fs::read("../certs/ca.pem")?,
    )?;
    ntex::server::build()
        .bind(
            "MTLS",
            "127.0.0.1:8443",
            SharedCfg::new("MTLS"),
            async move |_| {
                let trust = load_trust("../certs/ca.pem").expect("client trust anchors");
                mtls_service(config.clone(), trust)
            },
        )?
        .run()
        .await?;
    Ok(())
}
