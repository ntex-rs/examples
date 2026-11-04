//! A client transport that sends every call to the backend of its service.
use std::{collections::HashMap, rc::Rc};

use ntex::{Error, SharedCfg};
use ntex_grpc::client::{
    CallFailure, Client, ClientError, RequestContext, Response, StreamCall, StreamTransport,
    Transport,
};
use ntex_grpc::{GrpcStatus, MethodDef};
use ntex_h2::client as h2;

/// Routes calls by service name, e.g. `shop.Catalog`, to a backend client.
///
/// Generated clients own their transport, so clone the router for every
/// typed client. Clones share the routes and the connection pools.
#[derive(Clone, Default)]
pub struct Router {
    routes: Rc<HashMap<&'static str, Client>>,
}

impl Router {
    /// A router without routes, every call fails with `UNIMPLEMENTED`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends the calls of `service` to `client`.
    ///
    /// `service` is the full name with the package, use the generated
    /// `ServiceDef::NAME`.
    pub fn route(mut self, service: &'static str, client: Client) -> Self {
        Rc::make_mut(&mut self.routes).insert(service, client);
        self
    }

    /// Sends the calls of `service` to the server at `addr`.
    ///
    /// The h2 pool connects on the first call, so this does not touch the
    /// network.
    pub fn route_to(self, service: &'static str, addr: impl Into<String>) -> Self {
        let pool = h2::Client::builder(addr.into()).build(SharedCfg::new("GRPC-ROUTER"));
        self.route(service, Client::new(pool))
    }

    /// The backend for the service of method `M`.
    fn backend<M: MethodDef>(&self) -> Result<&Client, Error<ClientError>> {
        let path = M::PATH;
        let service = service_name(&path);
        self.routes.get(service).ok_or_else(|| {
            // Nothing is sent and trying again fails the same way.
            Error::from(ClientError::InvalidRequest(CallFailure::new(
                GrpcStatus::Unimplemented,
                format!("no backend for {service}"),
            )))
        })
    }
}

/// The service part of a request path: `/shop.Catalog/GetItem` gives
/// `shop.Catalog`.
pub fn service_name(path: &str) -> &str {
    path.trim_start_matches('/')
        .split_once('/')
        .map_or("", |(service, _)| service)
}

/// Unary calls.
///
/// Generic over the request state `St`, so `send_with(&state)` works too.
/// The built-in `Client` takes care of headers, timeouts and errors.
impl<M: MethodDef, St> Transport<M, St> for Router {
    type Error = Error<ClientError>;

    async fn request(
        &self,
        args: &M::Input,
        ctx: &mut RequestContext<'_, St>,
    ) -> Result<Response<M>, Self::Error> {
        Transport::<M, St>::request(self.backend::<M>()?, args, ctx).await
    }
}

/// Streaming calls.
///
/// Only the built-in transports can open a `StreamCall`, so pick a backend
/// and let it open the stream.
impl<M: MethodDef, St> StreamTransport<M, St> for Router {
    async fn open(
        &self,
        ctx: &mut RequestContext<'_, St>,
    ) -> Result<StreamCall, Error<ClientError>> {
        StreamTransport::<M, St>::open(self.backend::<M>()?, ctx).await
    }
}
