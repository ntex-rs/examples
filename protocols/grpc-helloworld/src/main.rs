//! gRPC server for the `helloworld.Greeter` service.
use grpc_helloworld::{ADDR, grpc_server};
use ntex::{SharedCfg, server::Server};

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let _ = env_logger::try_init();
    println!("gRPC server listening on {ADDR}");

    Server::builder()
        .bind("grpc", ADDR, SharedCfg::new("GRPC"), async |_| {
            grpc_server()
        })?
        .run()
        .await
}
