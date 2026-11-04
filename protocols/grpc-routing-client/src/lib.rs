//! A custom client transport that routes gRPC calls across backends.
//!
//! [`shop`] is generated from `shop.proto` by `ntex-grpc-codegen` (see
//! `generate.sh`). It has two services, `Catalog` and `Orders`, and each
//! runs on its own backend. [`router::Router`] is the transport of both
//! generated clients and sends every call to the backend of its service.
use std::convert::Infallible;
use std::sync::{Arc, atomic::AtomicU64, atomic::Ordering};

use ntex::ServiceFactory;
use ntex_grpc::server::{GrpcStatus, Response, ServerError};
use ntex_grpc::{ByteString, SendStream, ServiceDef, encode_grpc_message, server};

pub mod router;
pub mod shop;

use self::router::Router;
use self::shop::{Catalog, CatalogClient, Item, ItemRequest, ListRequest, Orders, OrdersClient};
use self::shop::{OrderReply, OrderRequest};

/// Address of the `shop.Catalog` backend.
pub const CATALOG_ADDR: &str = "127.0.0.1:50053";

/// Address of the `shop.Orders` backend.
pub const ORDERS_ADDR: &str = "127.0.0.1:50054";

/// SKUs and names of the catalog.
const ITEMS: [(&str, &str); 3] = [
    ("BOOK-42", "The Rust Programming Language"),
    ("PEN-7", "Fountain pen"),
    ("MUG-1", "Coffee mug"),
];

/// Implementation of the `shop.Catalog` service.
#[derive(Clone, Debug)]
pub struct CatalogServer {
    name: ByteString,
}

#[server(crate::shop::Catalog)]
impl CatalogServer {
    /// An unknown SKU ends the call with `NOT_FOUND`.
    #[method(GetItem)]
    async fn get_item(&self, req: ItemRequest) -> Result<Response<Item>, ServerError> {
        let Some((_, name)) = ITEMS.iter().find(|(sku, _)| *sku == req.sku) else {
            return Err(ServerError::new(
                GrpcStatus::NotFound,
                encode_grpc_message(&format!("no item {}", req.sku)),
                None,
            ));
        };
        let item = Item {
            sku: req.sku,
            name: (*name).into(),
            served_by: self.name.clone(),
        };
        Ok(item.into())
    }

    #[method(ListItems)]
    async fn list_items(&self, req: ListRequest, tx: SendStream<Item>) {
        for (sku, name) in ITEMS.iter().take(req.limit as usize) {
            let item = Item {
                sku: (*sku).into(),
                name: (*name).into(),
                served_by: self.name.clone(),
            };
            // the client went away
            if tx.send(&item).await.is_err() {
                return;
            }
        }
    }
}

impl ServiceFactory<(), server::ServerRequest> for CatalogServer {
    type Res = server::ServerResponse;
    type Error = server::ServerError;
    type Service = CatalogServer;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(self.clone())
    }
}

/// Implementation of the `shop.Orders` service.
#[derive(Clone, Debug)]
pub struct OrdersServer {
    name: ByteString,
    /// Shared by the workers, so order ids are unique.
    last_id: Arc<AtomicU64>,
}

#[server(crate::shop::Orders)]
impl OrdersServer {
    #[method(PlaceOrder)]
    async fn place_order(&self, _: OrderRequest) -> OrderReply {
        OrderReply {
            order_id: self.last_id.fetch_add(1, Ordering::Relaxed) + 1,
            served_by: self.name.clone(),
        }
    }
}

impl ServiceFactory<(), server::ServerRequest> for OrdersServer {
    type Res = server::ServerResponse;
    type Error = server::ServerError;
    type Service = OrdersServer;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(self.clone())
    }
}

/// The gRPC server of the catalog backend, its replies say `name`.
pub fn catalog_server(name: &str) -> server::GrpcServer {
    server::GrpcServer::new(CatalogServer { name: name.into() })
}

/// The gRPC server of the orders backend, its replies say `name`.
///
/// Pass the same `last_id` to every worker.
pub fn orders_server(name: &str, last_id: Arc<AtomicU64>) -> server::GrpcServer {
    server::GrpcServer::new(OrdersServer {
        name: name.into(),
        last_id,
    })
}

/// A router for the two backends.
pub fn router(catalog: impl Into<String>, orders: impl Into<String>) -> Router {
    Router::new()
        .route_to(Catalog::NAME, catalog)
        .route_to(Orders::NAME, orders)
}

/// Typed clients for both services, they share one router.
pub fn clients(router: &Router) -> (CatalogClient<Router>, OrdersClient<Router>) {
    (
        CatalogClient::new(router.clone()),
        OrdersClient::new(router.clone()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{
        server::test_server,
        time::{Seconds, timeout},
    };
    use ntex_grpc::{GrpcStatus, client::ClientError};

    /// STATEMENT: The parsed service name is the part of the path between
    /// the leading slash and the method.
    ///
    /// HOW:
    /// - Pass the generated paths of both services to `service_name()` and
    ///   check the names match `ServiceDef::NAME`.
    /// - Check a path without a method gives an empty name.
    #[test]
    fn service_name_comes_from_the_request_path() {
        use ntex_grpc::MethodDef;

        assert_eq!(
            router::service_name(&shop::CatalogGetItemMethod::PATH),
            Catalog::NAME
        );
        assert_eq!(
            router::service_name(&shop::OrdersPlaceOrderMethod::PATH),
            Orders::NAME
        );
        assert_eq!(router::service_name("/shop.Catalog"), "");
    }

    /// STATEMENT: Each call reaches the backend of its service, unary and
    /// streaming alike.
    ///
    /// HOW:
    /// - Serve the catalog and the orders backend on two test servers and
    ///   build both clients from one router.
    /// - Call `get_item`, `list_items` and `place_order`, and check the
    ///   replies and that `served_by` names the matching backend.
    /// - Call `get_item` with an unknown SKU and check the backend's
    ///   `NOT_FOUND` status and message reach the client.
    #[ntex::test]
    async fn calls_reach_the_backend_of_their_service() {
        let catalog = test_server(async |_| catalog_server("catalog-1"));
        let last_id = Arc::new(AtomicU64::new(0));
        let orders = test_server(async move |_| orders_server("orders-1", last_id.clone()));
        let router = router(catalog.addr().to_string(), orders.addr().to_string());
        let (catalog, orders) = clients(&router);

        timeout(Seconds(3), async {
            let item = catalog
                .get_item(&ItemRequest {
                    sku: "PEN-7".into(),
                })
                .send()
                .await
                .unwrap();
            assert_eq!(item.name, "Fountain pen");
            assert_eq!(item.served_by, "catalog-1");

            let err = catalog
                .get_item(&ItemRequest { sku: "NOPE".into() })
                .send()
                .await
                .unwrap_err();
            let ClientError::Status(status) = &*err else {
                panic!("unexpected error: {err:?}");
            };
            assert_eq!(status.code, GrpcStatus::NotFound);
            assert_eq!(err.message(), "no item NOPE");

            let mut rx = catalog
                .list_items(&ListRequest { limit: 2 })
                .open()
                .await
                .unwrap();
            let mut skus = Vec::new();
            while let Some(item) = rx.recv().await {
                let item = item.unwrap();
                assert_eq!(item.served_by, "catalog-1");
                skus.push(item.sku);
            }
            assert_eq!(skus, ["BOOK-42", "PEN-7"]);

            let order = orders
                .place_order(&OrderRequest {
                    sku: "PEN-7".into(),
                    quantity: 2,
                })
                .send()
                .await
                .unwrap();
            assert_eq!(order.order_id, 1);
            assert_eq!(order.served_by, "orders-1");
        })
        .await
        .unwrap();
    }

    /// STATEMENT: A call to a service without a route fails before
    /// anything is sent, with `UNIMPLEMENTED`.
    ///
    /// HOW:
    /// - Build a router with only the catalog route, pointing at a running
    ///   catalog backend.
    /// - Call `place_order` and check the error is `InvalidRequest` with
    ///   status `Unimplemented` and names the service.
    /// - Check the catalog route still works.
    #[ntex::test]
    async fn a_service_without_a_route_is_unimplemented() {
        let catalog = test_server(async |_| catalog_server("catalog-1"));
        let router = Router::new().route_to(Catalog::NAME, catalog.addr().to_string());
        let (catalog, orders) = clients(&router);

        timeout(Seconds(3), async {
            let err = orders
                .place_order(&OrderRequest {
                    sku: "PEN-7".into(),
                    quantity: 1,
                })
                .send()
                .await
                .unwrap_err();
            assert!(matches!(&*err, ClientError::InvalidRequest(_)));
            assert_eq!(err.status(), GrpcStatus::Unimplemented);
            assert_eq!(err.message(), "no backend for shop.Orders");

            let item = catalog
                .get_item(&ItemRequest {
                    sku: "MUG-1".into(),
                })
                .send()
                .await
                .unwrap();
            assert_eq!(item.served_by, "catalog-1");
        })
        .await
        .unwrap();
    }
}
