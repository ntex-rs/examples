//! Reserves stock with `inventory.Inventory/Reserve`.
//!
//! Usage: `grpc-custom-types-client SKU [QUANTITY]`
use std::time::Duration;

use grpc_custom_types::{
    ADDR, connect,
    inventory::ReserveRequest,
    types::{RequestId, Sku},
};

#[ntex::main]
async fn main() {
    let _ = env_logger::try_init();
    let mut args = std::env::args().skip(1);
    let sku = args.next().unwrap_or_else(|| "BOOK-42".to_string());
    let quantity = args
        .next()
        .map_or(1, |q| q.parse().expect("QUANTITY must be a number"));

    // The client can't build a request with an invalid SKU.
    let sku = match Sku::try_from(sku.as_str()) {
        Ok(sku) => sku,
        Err(err) => {
            eprintln!("{sku:?}: {err}");
            std::process::exit(2);
        }
    };
    let msg = ReserveRequest {
        request_id: RequestId::new(),
        sku,
        quantity,
    };

    let client = connect(ADDR);
    let mut req = client.reserve(&msg);
    req.timeout(Duration::from_secs(3));

    match req.send().await {
        Ok(res) if res.reserved => println!(
            "request {}: reserved {quantity} x {}, {} left",
            res.request_id, res.sku, res.remaining
        ),
        Ok(res) => println!(
            "request {}: not enough {}, {} left",
            res.request_id, res.sku, res.remaining
        ),
        Err(err) => {
            eprintln!("call failed with {:?}: {err}", err.status());
            std::process::exit(1);
        }
    }
}
