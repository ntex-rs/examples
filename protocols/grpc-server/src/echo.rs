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
pub struct EchoRequest {
    pub message: ::ntex_grpc::ByteString,
    pub repeat: u32,
    pub delay_ms: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct EchoReply {
    pub message: ::ntex_grpc::ByteString,
}

#[derive(Clone, PartialEq, Debug)]
pub struct StatsRequest {}

#[derive(Clone, PartialEq, Debug)]
pub struct StatsReply {
    pub calls: u64,
}

/// `Echo` service definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Echo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoMethods {
    UnaryEcho(EchoUnaryEchoMethod),
    ServerStreamingEcho(EchoServerStreamingEchoMethod),
    ClientStreamingEcho(EchoClientStreamingEchoMethod),
    BidiStreamingEcho(EchoBidiStreamingEchoMethod),
}

///  Echoes messages back, with one method of each kind.
#[derive(Debug, Clone)]
pub struct EchoClient<T>(T);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EchoUnaryEchoMethod;

impl ::ntex_grpc::MethodDef for EchoUnaryEchoMethod {
    const NAME: &'static str = "UnaryEcho";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/echo.Echo/UnaryEcho");
    type Input = EchoRequest;
    type Output = EchoReply;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = false;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EchoServerStreamingEchoMethod;

impl ::ntex_grpc::MethodDef for EchoServerStreamingEchoMethod {
    const NAME: &'static str = "ServerStreamingEcho";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/echo.Echo/ServerStreamingEcho");
    type Input = EchoRequest;
    type Output = EchoReply;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = true;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EchoClientStreamingEchoMethod;

impl ::ntex_grpc::MethodDef for EchoClientStreamingEchoMethod {
    const NAME: &'static str = "ClientStreamingEcho";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/echo.Echo/ClientStreamingEcho");
    type Input = EchoRequest;
    type Output = EchoReply;
    const CLIENT_STREAMING: bool = true;
    const SERVER_STREAMING: bool = false;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EchoBidiStreamingEchoMethod;

impl ::ntex_grpc::MethodDef for EchoBidiStreamingEchoMethod {
    const NAME: &'static str = "BidiStreamingEcho";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/echo.Echo/BidiStreamingEcho");
    type Input = EchoRequest;
    type Output = EchoReply;
    const CLIENT_STREAMING: bool = true;
    const SERVER_STREAMING: bool = true;
}
/// `Stats` service definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsMethods {
    GetStats(StatsGetStatsMethod),
}

///  Reports how many `Echo` calls the server handled.
#[derive(Debug, Clone)]
pub struct StatsClient<T>(T);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StatsGetStatsMethod;

impl ::ntex_grpc::MethodDef for StatsGetStatsMethod {
    const NAME: &'static str = "GetStats";
    const PATH: ::ntex_grpc::ByteString =
        ::ntex_grpc::ByteString::from_static("/echo.Stats/GetStats");
    type Input = StatsRequest;
    type Output = StatsReply;
    const CLIENT_STREAMING: bool = false;
    const SERVER_STREAMING: bool = false;
}

mod _priv_impl_echo {
    use super::*;

    impl ::ntex_grpc::Message for EchoRequest {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.message,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.repeat,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
                dst,
            );
            ::ntex_grpc::NativeType::serialize(
                &self.delay_ms,
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
            const STRUCT_NAME: &str = "EchoRequest";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => {
                        ::ntex_grpc::NativeType::deserialize(&mut self.message, tag, wire_type, src)
                            .map_err(|err| err.push(STRUCT_NAME, "message"))?
                    }
                    2 => {
                        ::ntex_grpc::NativeType::deserialize(&mut self.repeat, tag, wire_type, src)
                            .map_err(|err| err.push(STRUCT_NAME, "repeat"))?
                    }
                    3 => ::ntex_grpc::NativeType::deserialize(
                        &mut self.delay_ms,
                        tag,
                        wire_type,
                        src,
                    )
                    .map_err(|err| err.push(STRUCT_NAME, "delay_ms"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.message,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.repeat,
                2,
                ::ntex_grpc::types::DefaultValue::Default,
            ) + ::ntex_grpc::NativeType::serialized_len(
                &self.delay_ms,
                3,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for EchoRequest {
        #[inline]
        fn default() -> Self {
            Self {
                message: ::core::default::Default::default(),
                repeat: ::core::default::Default::default(),
                delay_ms: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for EchoReply {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.message,
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
            const STRUCT_NAME: &str = "EchoReply";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => {
                        ::ntex_grpc::NativeType::deserialize(&mut self.message, tag, wire_type, src)
                            .map_err(|err| err.push(STRUCT_NAME, "message"))?
                    }
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.message,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for EchoReply {
        #[inline]
        fn default() -> Self {
            Self {
                message: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::Message for StatsRequest {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {}

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
            const STRUCT_NAME: &str = "StatsRequest";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?;
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0
        }
    }

    impl ::std::default::Default for StatsRequest {
        #[inline]
        fn default() -> Self {
            Self {}
        }
    }

    impl ::ntex_grpc::Message for StatsReply {
        #[inline]
        fn write(&self, dst: &mut ::ntex_grpc::BytePages) {
            ::ntex_grpc::NativeType::serialize(
                &self.calls,
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
            const STRUCT_NAME: &str = "StatsReply";
            while !src.is_empty() {
                let (tag, wire_type) = ::ntex_grpc::encoding::decode_key(src)?;
                match tag {
                    1 => ::ntex_grpc::NativeType::deserialize(&mut self.calls, tag, wire_type, src)
                        .map_err(|err| err.push(STRUCT_NAME, "calls"))?,
                    _ => ::ntex_grpc::encoding::skip_field(wire_type, tag, src)?,
                }
            }
            Ok(())
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::ntex_grpc::NativeType::serialized_len(
                &self.calls,
                1,
                ::ntex_grpc::types::DefaultValue::Default,
            )
        }
    }

    impl ::std::default::Default for StatsReply {
        #[inline]
        fn default() -> Self {
            Self {
                calls: ::core::default::Default::default(),
            }
        }
    }

    impl ::ntex_grpc::ServiceDef for Echo {
        const NAME: &'static str = "echo.Echo";
        type Methods = EchoMethods;

        #[inline]
        fn method_by_name(name: &str) -> Option<Self::Methods> {
            use ::ntex_grpc::MethodDef;
            match name {
                EchoUnaryEchoMethod::NAME => Some(EchoMethods::UnaryEcho(EchoUnaryEchoMethod)),
                EchoServerStreamingEchoMethod::NAME => Some(EchoMethods::ServerStreamingEcho(
                    EchoServerStreamingEchoMethod,
                )),
                EchoClientStreamingEchoMethod::NAME => Some(EchoMethods::ClientStreamingEcho(
                    EchoClientStreamingEchoMethod,
                )),
                EchoBidiStreamingEchoMethod::NAME => {
                    Some(EchoMethods::BidiStreamingEcho(EchoBidiStreamingEchoMethod))
                }
                _ => None,
            }
        }
    }

    impl<T> EchoClient<T> {
        #[inline]
        /// Create new client instance
        pub fn new(transport: T) -> Self {
            Self(transport)
        }
    }

    impl<T> ::ntex_grpc::client::ClientInformation<T> for EchoClient<T> {
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

    impl<T> EchoClient<T> {
        ///  Unary: one request, one reply. Waits `delay_ms` before answering.
        pub fn unary_echo<'a>(
            &'a self,
            req: &'a super::EchoRequest,
        ) -> ::ntex_grpc::client::Request<'a, T, EchoUnaryEchoMethod> {
            ::ntex_grpc::client::Request::new(&self.0, req)
        }
    }

    impl<T> EchoClient<T> {
        ///  Server streaming: sends the message back `repeat` times.
        pub fn server_streaming_echo<'a>(
            &'a self,
            req: &'a super::EchoRequest,
        ) -> ::ntex_grpc::client::StreamRequest<
            'a,
            T,
            EchoServerStreamingEchoMethod,
            ::ntex_grpc::client::ServerStreaming<'a, super::EchoRequest>,
        > {
            ::ntex_grpc::client::StreamRequest::server_streaming(&self.0, req)
        }
    }

    impl<T> EchoClient<T> {
        ///  Client streaming: joins all received messages into one reply.
        pub fn client_streaming_echo(
            &self,
            tx: &::ntex_grpc::SendStream<super::EchoRequest>,
        ) -> ::ntex_grpc::client::StreamRequest<
            '_,
            T,
            EchoClientStreamingEchoMethod,
            ::ntex_grpc::client::ClientStreaming,
        > {
            ::ntex_grpc::client::StreamRequest::client_streaming(&self.0, tx)
        }
    }

    impl<T> EchoClient<T> {
        ///  Bidirectional streaming: answers every message as it arrives.
        pub fn bidi_streaming_echo(
            &self,
        ) -> ::ntex_grpc::client::StreamRequest<
            '_,
            T,
            EchoBidiStreamingEchoMethod,
            ::ntex_grpc::client::Bidi,
        > {
            ::ntex_grpc::client::StreamRequest::bidi(&self.0)
        }
    }
    impl ::ntex_grpc::ServiceDef for Stats {
        const NAME: &'static str = "echo.Stats";
        type Methods = StatsMethods;

        #[inline]
        fn method_by_name(name: &str) -> Option<Self::Methods> {
            use ::ntex_grpc::MethodDef;
            match name {
                StatsGetStatsMethod::NAME => Some(StatsMethods::GetStats(StatsGetStatsMethod)),
                _ => None,
            }
        }
    }

    impl<T> StatsClient<T> {
        #[inline]
        /// Create new client instance
        pub fn new(transport: T) -> Self {
            Self(transport)
        }
    }

    impl<T> ::ntex_grpc::client::ClientInformation<T> for StatsClient<T> {
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

    impl<T> StatsClient<T> {
        pub fn get_stats<'a>(
            &'a self,
            req: &'a super::StatsRequest,
        ) -> ::ntex_grpc::client::Request<'a, T, StatsGetStatsMethod> {
            ::ntex_grpc::client::Request::new(&self.0, req)
        }
    }
}
