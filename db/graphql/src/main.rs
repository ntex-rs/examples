//! Serve a MySQL-backed GraphQL API with Juniper and ntex.
//!
//! `src/handlers.rs` extracts JSON GraphQL requests and uses `web::block`
//! to execute the schema in `src/schemas/`. Application state shares the schema
//! and an `r2d2` MySQL pool; `/graphiql` provides an interactive interface.
//! Import the SQL schema and configure `.env` as described in the README.

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
