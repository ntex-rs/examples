//! Custom Rust types for gRPC message fields.
//!
//! [`inventory`] is generated from `inventory.proto` with `--map` options
//! (see `generate.sh`), so its `request_id` and `sku` fields use
//! [`RequestId`](types::RequestId) and [`Sku`](types::Sku) instead of
//! `Bytes` and `ByteString`.
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, Mutex},
};

use ntex::{ServiceFactory, SharedCfg};
use ntex_grpc::{client::Client, server};
use ntex_h2::client as h2;

pub mod inventory;
pub mod types;

#[cfg(test)]
#[path = "plain/inventory.rs"]
mod plain;

use self::inventory::{InventoryClient, ReserveReply, ReserveRequest};
use self::types::Sku;

/// Address the example server listens on and the client connects to.
pub const ADDR: &str = "127.0.0.1:50052";

/// Implementation of the `inventory.Inventory` service.
///
/// Clones share the stock, so every connection and worker sees the same
/// counts.
#[derive(Clone, Debug, Default)]
pub struct InventoryServer {
    stock: Arc<Mutex<HashMap<Sku, u32>>>,
}

impl InventoryServer {
    /// A server with the given stock.
    pub fn new(items: impl IntoIterator<Item = (Sku, u32)>) -> Self {
        Self {
            stock: Arc::new(Mutex::new(items.into_iter().collect())),
        }
    }

    /// A server with some stock to play with: 5 `BOOK-42` and 100 `PEN-7`.
    pub fn with_sample_stock() -> Self {
        Self::new([
            (Sku::try_from("BOOK-42").unwrap(), 5),
            (Sku::try_from("PEN-7").unwrap(), 100),
        ])
    }
}

#[server(crate::inventory::Inventory)]
impl InventoryServer {
    /// `req` is already validated: decoding fails for a malformed request id
    /// or SKU, and the server answers `INVALID_ARGUMENT` without calling this
    /// method.
    #[method(Reserve)]
    async fn reserve(&self, req: ReserveRequest) -> ReserveReply {
        let mut stock = self.stock.lock().unwrap();
        let (reserved, remaining) = match stock.get_mut(&req.sku) {
            Some(left) if *left >= req.quantity => {
                *left -= req.quantity;
                (true, *left)
            }
            Some(left) => (false, *left),
            None => (false, 0),
        };
        ReserveReply {
            request_id: req.request_id,
            sku: req.sku,
            reserved,
            remaining,
        }
    }
}

/// `GrpcServer` creates the service for every connection.
impl ServiceFactory<(), server::ServerRequest> for InventoryServer {
    type Res = server::ServerResponse;
    type Error = server::ServerError;
    type Service = InventoryServer;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(self.clone())
    }
}

/// The gRPC server for one connection, it serves `inventory.Inventory`.
pub fn grpc_server(inventory: InventoryServer) -> server::GrpcServer {
    server::GrpcServer::new(inventory)
}

/// Builds an `Inventory` client for `addr`.
pub fn connect(addr: impl Into<String>) -> InventoryClient<Client> {
    let pool = h2::Client::builder(addr.into()).build(SharedCfg::new("GRPC-CLIENT"));
    InventoryClient::new(Client::new(pool))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RequestId;
    use ntex::{
        server::test_server,
        time::{Seconds, timeout},
    };
    use ntex_grpc::{BytePages, Bytes, GrpcStatus, Message};

    fn encode(msg: &impl Message) -> Bytes {
        let mut buf = BytePages::default();
        msg.write(&mut buf);
        buf.freeze()
    }

    /// STATEMENT: A call carries the custom field types to the service and back.
    ///
    /// HOW:
    /// - Serve an `InventoryServer` with 5 `BOOK-42` and call `reserve` with a new `RequestId`.
    /// - Check the reply echoes the id and SKU and reserves 2 of 5 items.
    /// - Ask for 4 more and check the reservation fails with 3 items left.
    #[ntex::test]
    async fn reserve_round_trips_custom_types() {
        let srv = test_server(async |_| grpc_server(InventoryServer::with_sample_stock()));
        let client = connect(srv.addr().to_string());
        let sku = Sku::try_from("BOOK-42").unwrap();

        timeout(Seconds(3), async {
            let id = RequestId::new();
            let req = ReserveRequest {
                request_id: id,
                sku: sku.clone(),
                quantity: 2,
            };
            let res = client.reserve(&req).send().await.unwrap();
            assert_eq!(res.request_id, id);
            assert_eq!(res.sku, sku);
            assert!(res.reserved);
            assert_eq!(res.remaining, 3);

            let req = ReserveRequest {
                request_id: RequestId::new(),
                sku,
                quantity: 4,
            };
            let res = client.reserve(&req).send().await.unwrap();
            assert!(!res.reserved);
            assert_eq!(res.remaining, 3);
        })
        .await
        .unwrap();
    }

    /// STATEMENT: Custom field types keep the wire format of the proto types they replace.
    ///
    /// HOW:
    /// - Encode a typed `ReserveRequest` and decode it as the unmapped `plain` message.
    /// - Check `request_id` is the 16 UUID bytes and `sku` the same string.
    /// - Encode a typed request with the default id and SKU and check it is
    ///   byte-for-byte the plain request with empty fields, so defaults are left out.
    #[test]
    fn custom_types_keep_the_proto_wire_format() {
        let id = RequestId::new();
        let typed = ReserveRequest {
            request_id: id,
            sku: Sku::try_from("PEN-7").unwrap(),
            quantity: 3,
        };
        let plain = plain::ReserveRequest::read(&mut encode(&typed)).unwrap();
        assert_eq!(&plain.request_id[..], id.as_uuid().as_bytes());
        assert_eq!(plain.sku, "PEN-7");
        assert_eq!(plain.quantity, 3);

        let typed = ReserveRequest {
            request_id: RequestId::default(),
            sku: Sku::default(),
            quantity: 3,
        };
        let plain = plain::ReserveRequest {
            request_id: Bytes::new(),
            sku: "".into(),
            quantity: 3,
        };
        assert_eq!(encode(&typed), encode(&plain));
    }

    /// STATEMENT: Decoding rejects field values the custom types don't accept.
    ///
    /// HOW:
    /// - Encode plain requests with a 3-byte request id and with the SKU `book 42`.
    /// - Decode each as the typed `ReserveRequest` and check decoding fails.
    #[test]
    fn invalid_field_values_fail_to_decode() {
        let short_id = plain::ReserveRequest {
            request_id: Bytes::from_static(b"abc"),
            sku: "BOOK-42".into(),
            quantity: 1,
        };
        assert!(ReserveRequest::read(&mut encode(&short_id)).is_err());

        let bad_sku = plain::ReserveRequest {
            request_id: Bytes::new(),
            sku: "book 42".into(),
            quantity: 1,
        };
        assert!(ReserveRequest::read(&mut encode(&bad_sku)).is_err());
    }

    /// STATEMENT: The server answers `INVALID_ARGUMENT` when a request field fails to decode.
    ///
    /// HOW:
    /// - Serve an `InventoryServer` and call it with the unmapped `plain` client,
    ///   which can send any SKU.
    /// - Send the SKU `book 42` and check the call fails with `GrpcStatus::InvalidArgument`.
    #[ntex::test]
    async fn server_rejects_invalid_fields() {
        let srv = test_server(async |_| grpc_server(InventoryServer::with_sample_stock()));
        let pool = h2::Client::builder(srv.addr().to_string()).build(SharedCfg::new("TEST"));
        let client = plain::InventoryClient::new(Client::new(pool));

        timeout(Seconds(3), async {
            let req = plain::ReserveRequest {
                request_id: Bytes::new(),
                sku: "book 42".into(),
                quantity: 1,
            };
            let err = client.reserve(&req).send().await.unwrap_err();
            assert_eq!(err.status(), GrpcStatus::InvalidArgument);
        })
        .await
        .unwrap();
    }
}
