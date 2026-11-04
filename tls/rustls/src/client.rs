//! HTTPS client that verifies the server against the local CA.
//!
//! Usage: `rustls-client [URL] [CLIENT_IDENTITY_STEM]`. The optional stem, such as
//! `../certs/client`, loads `<stem>.pem` and `<stem>.key` as a client certificate.

use ntex::{SharedCfg, client::Client};
use rustls_example::client_config;
use tls_x509_example::{Error, load_identity};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "https://localhost:8443/".into());
    let identity = args.next().map(load_identity).transpose()?;
    let config = client_config(&std::fs::read("../certs/ca.pem")?, identity.as_ref())?;
    let client = Client::builder()
        .rustls(config)
        .build(SharedCfg::new("HTTPS"));
    let response = client.get(url.as_str()).send().await?;
    println!("Status: {}", response.status());
    println!("{}", String::from_utf8_lossy(&response.body().await?));
    Ok(())
}
