//! gRPC server for `helloworld.Greeter` over TLS.
//!
//! Run from `protocols/grpc-tls` after `../../tls/generate-certs.sh`:
//! `cargo run --bin grpc-tls-server`

use grpc_tls::{ADDR, grpc_tls_service, server_config};
use ntex::{SharedCfg, server::Server};
use tls_x509_example::{Error, load_identity};

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    let identity = load_identity("../../tls/certs/server")?;
    let config = server_config(&identity)?;
    println!("gRPC server listening on {ADDR} with TLS");

    Server::builder()
        .bind(
            "grpc-tls",
            ADDR,
            SharedCfg::new("GRPC-TLS"),
            async move |_| grpc_tls_service(config.clone()),
        )?
        .run()
        .await?;
    Ok(())
}
