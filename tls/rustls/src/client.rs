use std::{fs::File, io::BufReader, sync::Arc};

use ntex::{SharedCfg, client::Client};
use rustls::{ClientConfig, RootCertStore, sign::SingleCertAndKey};
use tls_x509_example::{Error, load_identity};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "https://localhost:8443/".into());
    let mut roots = RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut BufReader::new(File::open("../certs/ca.pem")?)) {
        roots.add(cert?)?;
    }
    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots);
    let mut config = if let Some(stem) = args.next() {
        let key = ntex::tls::rustls::certified_key(&load_identity(stem)?)?;
        builder.with_client_cert_resolver(Arc::new(SingleCertAndKey::from(key)))
    } else {
        builder.with_no_client_auth()
    };
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let client = Client::builder()
        .rustls(config)
        .build(SharedCfg::new("HTTPS"));
    let response = client.get(url.as_str()).send().await?;
    println!("Status: {}", response.status());
    println!("{}", String::from_utf8_lossy(&response.body().await?));
    Ok(())
}
