mod codec;
mod server;
mod tcp;
mod web;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=info,server=trace");
    }
    env_logger::init();

    println!("Started chat server");

    // Start chat server
    let server = server::start();

    let ws_srv = server.clone();
    let tcp_srv = server.clone();

    // Create server
    ntex::server::build()
        .bind(
            "tcp",
            "127.0.0.1:12345",
            ntex::SharedCfg::new("TCP"),
            async move |_| tcp::server(tcp_srv.clone()),
        )?
        .bind(
            "websockets",
            "127.0.0.1:8080",
            ntex::SharedCfg::new("WEBSOCKETS"),
            async move |_| web::server(ws_srv.clone()),
        )?
        .run()
        .await
}
