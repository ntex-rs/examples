use ntex::web::{self, middleware, App};

use async_ex2::appconfig::config_app;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=info");
    env_logger::init();

    web::server(async || {
        App::new()
            .configure(config_app)
            .middleware(middleware::Logger::default())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
