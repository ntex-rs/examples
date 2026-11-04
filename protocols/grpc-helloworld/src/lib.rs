//! A unary gRPC service with ntex-grpc.
//!
//! [`helloworld`] is generated from `helloworld.proto` by `ntex-grpc-codegen`
//! (see `generate.sh`). It has the messages, the `Greeter` service
//! definition and the typed `GreeterClient`. This crate adds the server
//! implementation and a helper that builds a client.
use std::convert::Infallible;

use ntex::{ServiceFactory, SharedCfg};
use ntex_grpc::{client::Client, server};
use ntex_h2::client as h2;

pub mod helloworld;

use self::helloworld::{GreeterClient, HelloReply, HelloRequest};

/// Address the example server listens on and the client connects to.
pub const ADDR: &str = "127.0.0.1:50051";

/// Implementation of the `helloworld.Greeter` service.
#[derive(Clone, Debug, Default)]
pub struct GreeterServer;

#[server(crate::helloworld::Greeter)]
impl GreeterServer {
    #[method(SayHello)]
    async fn say_hello(&self, req: HelloRequest) -> HelloReply {
        let name = if req.name.is_empty() { "world" } else { &req.name };
        HelloReply {
            message: format!("Hello {name}!").into(),
        }
    }
}

/// `GrpcServer` creates the service for every connection.
impl ServiceFactory<(), server::ServerRequest> for GreeterServer {
    type Res = server::ServerResponse;
    type Error = server::ServerError;
    type Service = GreeterServer;
    type InitError = Infallible;

    async fn create(&self, _: &()) -> Result<Self::Service, Self::InitError> {
        Ok(self.clone())
    }
}

/// The gRPC server for one connection, it serves `helloworld.Greeter`.
pub fn grpc_server() -> server::GrpcServer {
    server::GrpcServer::new(GreeterServer)
}

/// Builds a `Greeter` client for `addr`.
///
/// The h2 pool opens connections when the first call needs one, so this
/// does not touch the network.
pub fn connect(addr: impl Into<String>) -> GreeterClient<Client> {
    let pool = h2::Client::builder(addr.into()).build(SharedCfg::new("GRPC-CLIENT"));
    GreeterClient::new(Client::new(pool))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{
        server::test_server,
        time::{Seconds, timeout},
    };

    /// STATEMENT: The generated client calls the `Greeter` service and gets its typed reply.
    ///
    /// HOW:
    /// - Serve `grpc_server()` with a test server and build a client with `connect()`.
    /// - Call `say_hello` with a name and check the reply message and the gRPC
    ///   `content-type` response header within a timeout.
    #[ntex::test]
    async fn say_hello_returns_a_greeting() {
        let srv = test_server(async |_| grpc_server());
        let client = connect(srv.addr().to_string());
        timeout(Seconds(3), async {
            let res = client
                .say_hello(&HelloRequest {
                    name: "ntex".into(),
                })
                .send()
                .await
                .unwrap();
            assert_eq!(res.message, "Hello ntex!");
            assert_eq!(
                res.headers().get("content-type").unwrap(),
                "application/grpc"
            );
        })
        .await
        .unwrap();
    }

    /// STATEMENT: An empty name, the protobuf default, is greeted as `world`.
    ///
    /// HOW:
    /// - Serve `grpc_server()` with a test server.
    /// - Send a `HelloRequest` with an empty name and check the reply is
    ///   `Hello world!` within a timeout.
    #[ntex::test]
    async fn empty_name_greets_the_world() {
        let srv = test_server(async |_| grpc_server());
        let client = connect(srv.addr().to_string());
        timeout(Seconds(3), async {
            let res = client
                .say_hello(&HelloRequest { name: "".into() })
                .send()
                .await
                .unwrap();
            assert_eq!(res.message, "Hello world!");
        })
        .await
        .unwrap();
    }
}
