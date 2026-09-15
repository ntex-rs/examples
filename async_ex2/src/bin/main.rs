use ntex::web::{self, App, middleware};

use async_ex2::appconfig::config_app;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(config_app)
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("EX2"))?
    .run()
    .await
}
