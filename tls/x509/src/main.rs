//! Prints certificate metadata from PEM, DER, PKCS#12, a PEM identity, or an
//! `IdentitySource` string. Loading a certificate is not a trust check.

use std::{env, fs};

use ntex_tls::x509::{Certificate, Identity, IdentitySource, digest::Algorithm};
use tls_x509_example::{Error, load_identity};

fn main() -> Result<(), Error> {
    let args: Vec<_> = env::args().skip(1).collect();
    let cert = match args.as_slice() {
        [kind, path] if kind == "pem" => Certificate::from_pem(&fs::read(path)?)?,
        [kind, path] if kind == "der" => Certificate::from_der(fs::read(path)?)?,
        [kind, stem] if kind == "identity" => load_identity(stem)?.cert().clone(),
        [kind, path] if kind == "pkcs12" => {
            let password = env::var("PFX_PASSWORD").unwrap_or_default();
            Identity::from_pkcs12(&fs::read(path)?, &password)?.cert().clone()
        }
        [kind, source] if kind == "source" => {
            source.parse::<IdentitySource>()?.load()?.cert().clone()
        }
        _ => return Err(
            "usage: tls-x509-example <pem|der|identity|pkcs12|source> <path, stem, or store selector>"
                .into(),
        ),
    };
    println!("Subject: {}", cert.subject());
    println!("Issuer: {}", cert.issuer());
    println!("SAN: {:?}", cert.san());
    println!("Valid: {:?} .. {:?}", cert.not_before(), cert.not_after());
    println!("SHA-256: {}", cert.thumbprint(Algorithm::Sha256));
    println!("Parsing or loading an identity is not a trust check.");
    Ok(())
}
