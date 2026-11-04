//! Runs the two backends: `shop.Catalog` and `shop.Orders`, each on its own
//! address.
use std::sync::{Arc, atomic::AtomicU64};

use grpc_routing_client::{CATALOG_ADDR, ORDERS_ADDR, catalog_server, orders_server};
use ntex::{SharedCfg, server::Server};

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let _ = env_logger::try_init();
    println!("shop.Catalog backend listening on {CATALOG_ADDR}");
    println!("shop.Orders backend listening on {ORDERS_ADDR}");

    let last_id = Arc::new(AtomicU64::new(0));
    Server::builder()
        .bind(
            "catalog",
            CATALOG_ADDR,
            SharedCfg::new("GRPC"),
            async |_| catalog_server("catalog-1"),
        )?
        .bind(
            "orders",
            ORDERS_ADDR,
            SharedCfg::new("GRPC"),
            async move |_| orders_server("orders-1", last_id.clone()),
        )?
        .run()
        .await
}
