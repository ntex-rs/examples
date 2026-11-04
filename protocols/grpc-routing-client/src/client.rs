//! Calls both services through one router and prints which backend
//! answered each call.
use std::time::Duration;

use grpc_routing_client::shop::{ItemRequest, ListRequest, OrderRequest};
use grpc_routing_client::{CATALOG_ADDR, ORDERS_ADDR, clients, router};

#[ntex::main]
async fn main() {
    let _ = env_logger::try_init();
    let router = router(CATALOG_ADDR, ORDERS_ADDR);
    let (catalog, orders) = clients(&router);

    let msg = ItemRequest {
        sku: "BOOK-42".into(),
    };
    let mut req = catalog.get_item(&msg);
    req.timeout(Duration::from_secs(3));
    let item = req.send().await.unwrap_or_else(|err| fail(err));
    println!(
        "GetItem    {} {:?} from {}",
        item.sku, item.name, item.served_by
    );

    let msg = ListRequest { limit: 10 };
    let mut rx = catalog
        .list_items(&msg)
        .open()
        .await
        .unwrap_or_else(|err| fail(err));
    while let Some(item) = rx.recv().await {
        let item = item.unwrap_or_else(|err| fail(err));
        println!(
            "ListItems  {} {:?} from {}",
            item.sku, item.name, item.served_by
        );
    }

    let msg = OrderRequest {
        sku: "BOOK-42".into(),
        quantity: 1,
    };
    let order = orders
        .place_order(&msg)
        .send()
        .await
        .unwrap_or_else(|err| fail(err));
    println!("PlaceOrder #{} from {}", order.order_id, order.served_by);
}

fn fail<E: std::fmt::Display>(err: E) -> ! {
    eprintln!("call failed: {err}");
    std::process::exit(1);
}
