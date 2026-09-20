#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use ntex::web::{self, App, middleware};
use ntex_identity::{CookieIdentityPolicy, IdentityService};

mod auth_handler;
mod email_service;
mod errors;
mod invitation_handler;
mod models;
mod register_handler;
mod schema;
mod utils;

type AppState = web::AppState<models::Pool>;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create the database connection pool.
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool: models::Pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let domain: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    // Start the HTTP server.
    web::server(async move |_| {
        App::new()
            // Enable request logging.
            .middleware(middleware::Logger::default())
            .middleware(IdentityService::new(
                CookieIdentityPolicy::new(utils::SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/")
                    .domain(domain.as_str())
                    .max_age_time(time::Duration::days(1))
                    .secure(false), // Enable this when serving over HTTPS.
            ))
            .with_config(
                web::WebAppConfig::new().set_state(web::types::JsonConfig::default().limit(4096)),
            )
            // Register all API endpoints under `/api`.
            .service(
                web::scope("/api").service((
                    web::resource("/invitation")
                        .route(web::post().to_with_state(invitation_handler::post_invitation)),
                    web::resource("/register/{invitation_id}")
                        .route(web::post().to_with_state(register_handler::register_user)),
                    web::resource("/auth")
                        .route(web::post().to_with_state(auth_handler::login))
                        .route(web::delete().to(auth_handler::logout))
                        .route(web::get().to(auth_handler::get_me)),
                )),
            )
            .build_with(AppState::new(pool.clone()))
    })
    .bind("127.0.0.1:3000", ntex::SharedCfg::new("AUTH"))?
    .run()
    .await
}
