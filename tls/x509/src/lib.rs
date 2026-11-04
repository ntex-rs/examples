use std::{fs, path::Path, time::SystemTime};

use ntex::io::IoRef;
use ntex_tls::x509::{self, CaCerts, Certificate, Identity, PeerCerts, auth};
use ntex_tls::{PeerCertChainDer, PeerCertDer};

pub type Error = Box<dyn std::error::Error>;

/// Loads a leaf-first PEM chain and an unencrypted PKCS#8 key with the same file stem.
pub fn load_identity(stem: impl AsRef<Path>) -> Result<Identity, Error> {
    let stem = stem.as_ref();
    Ok(Identity::from_pem(
        &fs::read(stem.with_extension("pem"))?,
        &fs::read(stem.with_extension("key"))?,
    )?)
}

/// Loads explicitly configured client trust anchors, rejecting partial imports.
pub fn load_trust(path: impl AsRef<Path>) -> Result<CaCerts, Error> {
    let certs = Certificate::from_pem_multi(&fs::read(path)?)?;
    let count = certs.len();
    let trust = CaCerts::from_certs(certs);
    if count == 0 || trust.len() != count {
        return Err("not all client trust anchors could be loaded".into());
    }
    Ok(trust)
}

/// Retrieves portable certificate metadata without assuming it is trusted.
pub fn peer_certs(io: &IoRef) -> Result<Option<PeerCerts>, x509::Error> {
    let Some(leaf) = io.query::<PeerCertDer>().as_ref().cloned() else {
        return Ok(None);
    };
    let chain = io
        .query::<PeerCertChainDer>()
        .as_ref()
        .cloned()
        .map(|item| item.0)
        .unwrap_or_default();
    PeerCerts::from_der(leaf.0, chain).map(Some)
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("a client certificate is required")]
    MissingCertificate,
    #[error(transparent)]
    Certificate(#[from] x509::Error),
    #[error("client certificate is not trusted: {0:?}")]
    Trust(x509::TrustError),
    #[error("client identity is not authorized: {0:?}")]
    Name(auth::Failure),
}

/// Verifies client trust and the example's URI identity; returns the verified path's expiry.
pub fn authorize(peer: &PeerCerts, trust: &CaCerts) -> Result<SystemTime, AuthError> {
    let expires = trust
        .is_trusted(peer, SystemTime::now())
        .map_err(AuthError::Trust)?;
    auth::name_matches(
        peer.leaf(),
        auth::NameField::Uri,
        "spiffe://ntex.example/client",
    )
    .map_err(AuthError::Name)?;
    Ok(expires)
}

/// Rejects missing certificates before applying the client trust and identity policy.
pub fn authenticate(io: &IoRef, trust: &CaCerts) -> Result<SystemTime, AuthError> {
    let peer = peer_certs(io)?.ok_or(AuthError::MissingCertificate)?;
    authorize(&peer, trust)
}

/// Bounds the connection lifetime by its verified certificate path's expiry.
///
/// The timer is cancelled when the connection disconnects. Durations beyond the
/// timer's range are capped, closing earlier rather than extending authorization.
pub fn close_at_expiry(io: &IoRef, expires: SystemTime) {
    let io = io.clone();
    let remaining = expires
        .duration_since(SystemTime::now())
        .unwrap_or_default();
    ntex::rt::spawn(async move {
        let deadline = ntex::time::sleep(ntex::time::Millis::from(remaining));
        if matches!(
            ntex::util::select(deadline, io.on_disconnect()).await,
            ntex::util::Either::Left(_)
        ) {
            io.terminate();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex_tls::x509::testgen::{self, San};

    /// STATEMENT: Client authorization requires both a trusted chain and the allowed URI.
    ///
    /// HOW:
    /// - Create a root and clients with matching and different URI SANs.
    /// - Accept the matching client, reject the other name, and reject an empty trust store.
    #[test]
    fn client_trust_and_name() {
        let ca_key = testgen::key();
        let ca = testgen::cert("ca", true, None, &ca_key, &San::default(), (-1, 1));
        let trust = CaCerts::from_certs([ca.clone()]);
        let client = |uri| {
            PeerCerts::new(
                testgen::cert(
                    "client",
                    false,
                    Some((&ca, &ca_key)),
                    &testgen::key(),
                    &San {
                        uri: &[uri],
                        ..San::default()
                    },
                    (-1, 1),
                ),
                vec![],
            )
        };
        let peer = client("spiffe://ntex.example/client");
        assert!(authorize(&peer, &trust).is_ok());
        assert!(matches!(
            authorize(&client("spiffe://ntex.example/other"), &trust),
            Err(AuthError::Name(_))
        ));
        let empty = CaCerts::from_certs(Vec::<Certificate>::new());
        assert!(matches!(authorize(&peer, &empty), Err(AuthError::Trust(_))));
    }

    /// STATEMENT: Missing peer certificates are rejected and expired sessions are terminated.
    ///
    /// HOW:
    /// - Create plain `Io` with a live peer and check `MissingCertificate`.
    /// - Apply an elapsed expiry and wait for disconnection within a bounded timeout.
    #[ntex::test]
    async fn missing_certificate_and_expiry() {
        let (transport, _peer) = ntex::io::testing::IoTest::create();
        let io = ntex::io::Io::new(transport, ntex::SharedCfg::default());
        assert!(matches!(
            authenticate(&io, &CaCerts::default()),
            Err(AuthError::MissingCertificate)
        ));
        assert!(!io.is_closed());
        close_at_expiry(&io, SystemTime::UNIX_EPOCH);
        ntex::time::timeout(ntex::time::Millis(500), io.on_disconnect())
            .await
            .unwrap();
        assert!(io.is_closed());
    }
}
