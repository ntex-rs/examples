//! gRPC server for the `echo.Echo` and `echo.Stats` services.
use std::sync::{Arc, atomic::AtomicU64};

use grpc_server::{ADDR, grpc_server, server_cfg};
use ntex::server::Server;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let _ = env_logger::try_init();
    println!("gRPC server listening on {ADDR}");

    // one counter for all workers and connections
    let calls = Arc::new(AtomicU64::new(0));
    Server::builder()
        .bind("grpc", ADDR, server_cfg(), async move |_| {
            grpc_server(calls.clone())
        })?
        .run()
        .await
}
