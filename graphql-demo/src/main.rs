#[macro_use]
extern crate juniper;

use std::sync::Arc;

use ntex::web::{self, App, middleware};

use crate::db::get_db_pool;
use crate::handlers::{AppState, register};
use crate::schemas::root::create_schema;

mod db;
mod handlers;
mod schemas;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let pool = get_db_pool();

    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(register)
            .default_service(web::to(|| async { "404" }))
            .build_with(AppState {
                pool: pool.clone(),
                schema: Arc::new(create_schema()),
            })
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("GQL"))?
    .run()
    .await
}
