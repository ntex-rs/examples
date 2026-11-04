use ntex::{client::Client, SharedCfg};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tls_x509_example::{load_identity, Error};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "https://localhost:8443/".into());
    let mut config = SslConnector::builder(SslMethod::tls())?;
    config.set_verify(SslVerifyMode::PEER);
    config.set_ca_file("../certs/ca.pem")?;
    config.set_alpn_protos(b"\x02h2\x08http/1.1")?;
    if let Some(stem) = args.next() {
        ntex::tls::openssl::set_identity(&mut config, &load_identity(stem)?)?;
    }
    let client = Client::builder()
        .openssl(config.build())
        .build(SharedCfg::new("HTTPS"));
    let response = client.get(url.as_str()).send().await?;
    println!("Status: {}", response.status());
    println!("{}", String::from_utf8_lossy(&response.body().await?));
    Ok(())
}
