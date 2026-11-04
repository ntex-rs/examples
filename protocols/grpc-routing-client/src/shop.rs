#![allow(
    dead_code,
    unused_mut,
    unused_variables,
    clippy::identity_op,
    clippy::too_many_lines,
    clippy::derivable_impls,
    clippy::unit_arg,
    clippy::derive_partial_eq_without_eq,
    clippy::manual_range_patterns,
    clippy::default_trait_access,
    clippy::semicolon_if_nothing_returned,
    clippy::doc_markdown,
    clippy::wildcard_imports
)]
//! DO NOT MODIFY. Auto-generated file

#[derive(Clone, PartialEq, Debug)]
pub struct ItemRequest {
    pub sku: ::ntex_grpc::ByteString,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ListRequest {
    pub limit: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Item {
    pub sku: ::ntex_grpc::ByteString,
    pub name: ::ntex_grpc::ByteString,
    ///  Name of the backend that answered.
    pub served_by: ::ntex_grpc::ByteString,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OrderRequest {
    pub sku: ::ntex_grpc::ByteString,
    pub quantity: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OrderReply {
    pub order_id: u64,
    ///  Name of the backend that answered.
    pub served_by: ::ntex_grpc::ByteString,
}

/// `Catalog` service definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Catalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogMethods {
    GetItem(CatalogGetItemMethod),
    ListItems(CatalogListItemsMethod),
}

///  Item lookups. The example runs it on its own backend.
#[derive(Debug, Clone)]
pub struct CatalogClient<T>(T);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CatalogGetItemMethod;

impl ::ntex_grpc::MethodDef for CatalogGetItemMethod {
    const NAME: &'static str = "GetItem";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/shop.Catalog/GetItem");
    type Input = ItemRequest;
    type Output = Item;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = false;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CatalogListItemsMethod;

impl ::ntex_grpc::MethodDef for CatalogListItemsMethod {
    const NAME: &'static str = "ListItems";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/shop.Catalog/ListItems");
    type Input = ListRequest;
    type Output = Item;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = true;
}
/// `Orders` service definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orders;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdersMethods {
    PlaceOrder(OrdersPlaceOrderMethod),
}

///  Order taking. The example runs it on another backend.
#[derive(Debug, Clone)]
pub struct OrdersClient<T>(T);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrdersPlaceOrderMethod;

impl ::ntex_grpc::MethodDef for OrdersPlaceOrderMethod {
    const NAME: &'static str = "PlaceOrder";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/shop.Orders/PlaceOrder");
    type Input = OrderRequest;
    type Output = OrderReply;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = false;
}

mod _priv_impl_shop {
    use super::*;

    impl ::ntex_grpc::Message for ItemRequest {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.sku,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
        }

        #[inline]
        fn read(
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<Self, ::ntex_grpc::DecodeError> {
            let mut msg = Self::default();
            ::ntex_grpc::Message::merge_from(&mut msg, src)?;
            Ok(msg)
        }

        #[inline]
        fn merge_from(
            &mut self,
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<(), ::ntex_grpc::DecodeError> {
            const STRUCT_NAME: &str = "ItemRequest";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(&mut self.sku, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "sku"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.sku,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for ItemRequest {
        #[inline]
        fn default() -> Self {
            Self {
                sku: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for ListRequest {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.limit,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
        }

        #[inline]
        fn read(
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<Self, ::ntex_grpc::DecodeError> {
            let mut msg = Self::default();
            ::ntex_grpc::Message::merge_from(&mut msg, src)?;
            Ok(msg)
        }

        #[inline]
        fn merge_from(
            &mut self,
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<(), ::ntex_grpc::DecodeError> {
            const STRUCT_NAME: &str = "ListRequest";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(&mut self.limit, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "limit"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.limit,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for ListRequest {
        #[inline]
        fn default() -> Self {
            Self {
                limit: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for Item {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.sku,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.name,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.served_by,
                3,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
        }

        #[inline]
        fn read(
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<Self, ::ntex_grpc::DecodeError> {
            let mut msg = Self::default();
            ::ntex_grpc::Message::merge_from(&mut msg, src)?;
            Ok(msg)
        }

        #[inline]
        fn merge_from(
            &mut self,
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<(), ::ntex_grpc::DecodeError> {
            const STRUCT_NAME: &str = "Item";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(&mut self.sku, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "sku"))?,
                    2 => ::ntex_grpc::NativeType::deserialize(&mut self.name, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "name"))?,
                    3 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.served_by,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "served_by"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.sku,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.name,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.served_by,
                3,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for Item {
        #[inline]
        fn default() -> Self {
            Self {
                sku: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                served_by: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for OrderRequest {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.sku,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.quantity,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
        }

        #[inline]
        fn read(
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<Self, ::ntex_grpc::DecodeError> {
            let mut msg = Self::default();
            ::ntex_grpc::Message::merge_from(&mut msg, src)?;
            Ok(msg)
        }

        #[inline]
        fn merge_from(
            &mut self,
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<(), ::ntex_grpc::DecodeError> {
            const STRUCT_NAME: &str = "OrderRequest";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(&mut self.sku, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "sku"))?,
                    2 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.quantity,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "quantity"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.sku,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.quantity,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for OrderRequest {
        #[inline]
        fn default() -> Self {
            Self {
                sku: ::core::default::Default::default(),
                quantity: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for OrderReply {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.order_id,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.served_by,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
        }

        #[inline]
        fn read(
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<Self, ::ntex_grpc::DecodeError> {
            let mut msg = Self::default();
            ::ntex_grpc::Message::merge_from(&mut msg, src)?;
            Ok(msg)
        }

        #[inline]
        fn merge_from(
            &mut self,
            src: &mut ::ntex_grpc::Bytes,
        ) -> ::std::result::Result<(), ::ntex_grpc::DecodeError> {
            const STRUCT_NAME: &str = "OrderReply";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.order_id,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "order_id"))?,
                    2 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.served_by,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "served_by"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.order_id,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.served_by,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for OrderReply {
        #[inline]
        fn default() -> Self {
            Self {
                order_id: ::core::default::Default::default(),
                served_by: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::ServiceDef for Catalog {
        const NAME: &'static str = "shop.Catalog";
        type Methods = CatalogMethods;

        #[inline]
        fn method_by_name(name: &str) -> Option<Self::Methods> {
            use ::ntex_grpc::MethodDef;
            match name {
                CatalogGetItemMethod::NAME => Some(CatalogMethods::GetItem(CatalogGetItemMethod)),
                CatalogListItemsMethod::NAME => {
                    Some(CatalogMethods::ListItems(CatalogListItemsMethod))
                }
                _ => None,
            }
        }
    }

    impl<T> CatalogClient<T> {
        #[inline]
        /// Create new client instance
        pub fn new(transport: T) -> Self {
            Self(transport)
        }
    }

    impl<T> ::ntex_grpc::client::ClientInformation<T> for CatalogClient<T> {
        #[inline]
        /// Create new client instance
        fn create(transport: T) -> Self {
            Self(transport)
        }

        #[inline]
        /// Get referece to underlying transport
        fn transport(&self) -> &T {
            &self.0
        }

        #[inline]
        /// Get mut referece to underlying transport
        fn transport_mut(&mut self) -> &mut T {
            &mut self.0
        }

        #[inline]
        /// Consume client and return inner transport
        fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T> CatalogClient<T> {
        ///  Returns one item by its SKU.
        pub fn get_item<'a>(
            &'a self,
            req: &'a super::ItemRequest,
        ) -> ::ntex_grpc::client::Request<'a, T, CatalogGetItemMethod> {
            ::ntex_grpc::client::Request::new(&self.0, req)
        }
    }

    impl<T> CatalogClient<T> {
        ///  Streams the items of the catalog, at most `limit` of them.
        pub fn list_items<'a>(
            &'a self,
            req: &'a super::ListRequest,
        ) -> ::ntex_grpc::client::StreamRequest<
            'a,
            T,
            CatalogListItemsMethod,
            ::ntex_grpc::client::ServerStreaming<'a, super::ListRequest>,
        > {
            ::ntex_grpc::client::StreamRequest::server_streaming(&self.0, req)
        }
    }
    impl ::ntex_grpc::ServiceDef for Orders {
        const NAME: &'static str = "shop.Orders";
        type Methods = OrdersMethods;

        #[inline]
        fn method_by_name(name: &str) -> Option<Self::Methods> {
            use ::ntex_grpc::MethodDef;
            match name {
                OrdersPlaceOrderMethod::NAME => {
                    Some(OrdersMethods::PlaceOrder(OrdersPlaceOrderMethod))
                }
                _ => None,
            }
        }
    }

    impl<T> OrdersClient<T> {
        #[inline]
        /// Create new client instance
        pub fn new(transport: T) -> Self {
            Self(transport)
        }
    }

    impl<T> ::ntex_grpc::client::ClientInformation<T> for OrdersClient<T> {
        #[inline]
        /// Create new client instance
        fn create(transport: T) -> Self {
            Self(transport)
        }

        #[inline]
        /// Get referece to underlying transport
        fn transport(&self) -> &T {
            &self.0
        }

        #[inline]
        /// Get mut referece to underlying transport
        fn transport_mut(&mut self) -> &mut T {
            &mut self.0
        }

        #[inline]
        /// Consume client and return inner transport
        fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T> OrdersClient<T> {
        ///  Places an order for an item.
        pub fn place_order<'a>(
            &'a self,
            req: &'a super::OrderRequest,
        ) -> ::ntex_grpc::client::Request<'a, T, OrdersPlaceOrderMethod> {
            ::ntex_grpc::client::Request::new(&self.0, req)
        }
    }
}
