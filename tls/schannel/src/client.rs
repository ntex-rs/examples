//! HTTPS client using Windows Schannel and Windows server-trust policy.
//!
//! Usage: `schannel-client [URL] [CLIENT_IDENTITY_STEM]`. `TLS_IDENTITY`, when set,
//! takes precedence over the stem and selects the client certificate.

#[cfg(windows)]
#[ntex::main]
async fn main() -> Result<(), tls_x509_example::Error> {
    use ntex::tls::schannel::{ClientConfig, TlsConnector};
    use ntex::tls::x509::IdentitySource;
    use ntex::{SharedCfg, client::Client};
    use tls_x509_example::load_identity;

    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "https://localhost:8443/".into());
    let mut config = ClientConfig::new();
    if let Ok(source) = std::env::var("TLS_IDENTITY") {
        let identity = source.parse::<IdentitySource>()?.load()?;
        config = config.set_client_cert(identity.schannel().clone());
    } else if let Some(stem) = args.next() {
        config = config.set_client_cert(load_identity(stem)?.schannel().clone());
    }
    let client = Client::builder()
        .secure_connector(TlsConnector::with_config(config))
        .build(SharedCfg::new("HTTPS"));
    let response = client.get(url.as_str()).send().await?;
    println!("Status: {}", response.status());
    println!("{}", String::from_utf8_lossy(&response.body().await?));
    Ok(())
}

#[cfg(not(windows))]
fn main() -> Result<(), &'static str> {
    Err("Schannel requires Windows")
}
