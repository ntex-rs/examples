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
pub struct ReserveRequest {
    ///  Client-chosen request id, a 16-byte UUID.
    pub request_id: ::ntex_grpc::Bytes,
    ///  Stock keeping unit, e.g. "BOOK-42".
    pub sku: ::ntex_grpc::ByteString,
    pub quantity: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ReserveReply {
    ///  The request id of the reservation.
    pub request_id: ::ntex_grpc::Bytes,
    pub sku: ::ntex_grpc::ByteString,
    pub reserved: bool,
    ///  Items left after the reservation.
    pub remaining: u32,
}

/// `Inventory` service definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inventory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryMethods {
    Reserve(InventoryReserveMethod),
}

///  Stock reservations.
#[derive(Debug, Clone)]
pub struct InventoryClient<T>(T);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InventoryReserveMethod;

impl ::ntex_grpc::MethodDef for InventoryReserveMethod {
    const NAME: &'static str = "Reserve";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/inventory.Inventory/Reserve");
    type Input = ReserveRequest;
    type Output = ReserveReply;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = false;
}

mod _priv_impl_inventory {
    use super::*;

    impl ::ntex_grpc::Message for ReserveRequest {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.request_id,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.sku,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.quantity,
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
            const STRUCT_NAME: &str = "ReserveRequest";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.request_id,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "request_id"))?,
                    2 => ::ntex_grpc::NativeType::deserialize(&mut self.sku, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "sku"))?,
                    3 => ::ntex_grpc::NativeType::deserialize(
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
                &self.request_id,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.sku,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.quantity,
                3,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for ReserveRequest {
        #[inline]
        fn default() -> Self {
            Self {
                request_id: ::core::default::Default::default(),
                sku: ::core::default::Default::default(),
                quantity: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for ReserveReply {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.request_id,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.sku,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.reserved,
                3,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.remaining,
                4,
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
            const STRUCT_NAME: &str = "ReserveReply";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.request_id,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "request_id"))?,
                    2 => ::ntex_grpc::NativeType::deserialize(&mut self.sku, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "sku"))?,
                    3 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.reserved,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "reserved"))?,
                    4 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.remaining,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "remaining"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.request_id,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.sku,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.reserved,
                3,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.remaining,
                4,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for ReserveReply {
        #[inline]
        fn default() -> Self {
            Self {
                request_id: ::core::default::Default::default(),
                sku: ::core::default::Default::default(),
                reserved: ::core::default::Default::default(),
                remaining: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::ServiceDef for Inventory {
        const NAME: &'static str = "inventory.Inventory";
        type Methods = InventoryMethods;

        #[inline]
        fn method_by_name(name: &str) -> Option<Self::Methods> {
            use ::ntex_grpc::MethodDef;
            match name {
                InventoryReserveMethod::NAME => {
                    Some(InventoryMethods::Reserve(InventoryReserveMethod))
                }
                _ => None,
            }
        }
    }

    impl<T> InventoryClient<T> {
        #[inline]
        /// Create new client instance
        pub fn new(transport: T) -> Self {
            Self(transport)
        }
    }

    impl<T> ::ntex_grpc::client::ClientInformation<T> for InventoryClient<T> {
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

    impl<T> InventoryClient<T> {
        ///  Reserves `quantity` items of `sku` if there is enough stock.
        pub fn reserve<'a>(
            &'a self,
            req: &'a super::ReserveRequest,
        ) -> ::ntex_grpc::client::Request<'a, T, InventoryReserveMethod> {
            ::ntex_grpc::client::Request::new(&self.0, req)
        }
    }
}
