use ntex::tls::x509::{Certificate, Identity};
use ntex_tls::x509::testgen::{self, San};

/// Generates short-lived test identities entirely in memory.
pub fn identities() -> (Certificate, Identity, Identity) {
    let ca_key = testgen::key();
    let ca = testgen::cert("test CA", true, None, &ca_key, &San::default(), (-1, 1));
    let identity = |name, san: San<'_>| {
        let key = testgen::key();
        let certificate = testgen::cert(name, false, Some((&ca, &ca_key)), &key, &san, (-1, 1));
        Identity::from_der(&[certificate.der(), ca.der()], &key.pkcs8_der()).unwrap()
    };
    let server = identity(
        "localhost",
        San {
            dns: &["localhost"],
            ..San::default()
        },
    );
    let client = identity(
        "client",
        San {
            uri: &["spiffe://ntex.example/client"],
            ..San::default()
        },
    );
    (ca, server, client)
}
