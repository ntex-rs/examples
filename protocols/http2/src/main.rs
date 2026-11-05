//! HTTP/2 server with the `/hello`, `/echo` and `/count` routes.
use http2::{ADDR, handler, server_cfg};
use ntex::server::Server;
use ntex_h2::server;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let _ = env_logger::try_init();
    println!("HTTP/2 server listening on {ADDR}");

    // plain TCP, clients must use HTTP/2 with prior knowledge
    Server::builder()
        .bind("h2", ADDR, server_cfg(), async |_| {
            server::Server::new(handler())
        })?
        .run()
        .await
}
