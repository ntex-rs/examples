#[macro_use]
extern crate juniper;

use ntex::web::{self, middleware, App};

use crate::db::get_db_pool;
use crate::handlers::register;

mod db;
mod handlers;
mod schemas;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "ntex=info,info");
    env_logger::init();

    let pool = get_db_pool();

    web::server(move || {
        App::new()
            .state(pool.clone())
            .wrap(middleware::Logger::default())
            .configure(register)
            .default_service(web::to(|| async { "404" }))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
