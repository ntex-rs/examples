//! gRPC server for the `inventory.Inventory` service.
use grpc_custom_types::{ADDR, InventoryServer, grpc_server};
use ntex::{SharedCfg, server::Server};

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let _ = env_logger::try_init();
    println!("gRPC server listening on {ADDR}");

    // One stock for all workers and connections.
    let inventory = InventoryServer::with_sample_stock();

    Server::builder()
        .bind("grpc", ADDR, SharedCfg::new("GRPC"), async move |_| {
            grpc_server(inventory.clone())
        })?
        .run()
        .await
}
