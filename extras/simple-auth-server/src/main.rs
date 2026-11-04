//! Authenticate invited users with PostgreSQL and identity cookies.
//!
//! Look at the `/api` scope for routes and the handler modules for
//! `web::block` database work, SparkPost email, and Argon2 password hashing.
//! Configure the database and email environment variables, apply Diesel
//! migrations, then run `cargo run` to listen on <http://127.0.0.1:3000>.

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

fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.service(
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
    );
}

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
            .configure(config)
            .build_with(AppState::new(pool.clone()))
    })
    .bind("127.0.0.1:3000", ntex::SharedCfg::new("AUTH"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};
    use serde_json::json;

    fn state() -> AppState {
        // Authentication and validation paths under test must not open database connections.
        let pool = models::Pool::builder()
            .max_size(1)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::<PgConnection>::new(
                "postgres://localhost/unused",
            ));
        AppState::new(pool)
    }

    /// STATEMENT: The current-user endpoint rejects a request without an identity cookie.
    ///
    /// HOW:
    /// - Initialize the real API routes with identity middleware and a pool that opens no connections.
    /// - Request `GET /api/auth` without cookies and check for `401` and the JSON error body.
    #[ntex::test]
    async fn unauthenticated_user_is_rejected() {
        let app = test::init_service_st(
            state(),
            App::new()
                .middleware(IdentityService::new(
                    CookieIdentityPolicy::new(&[0; 32]).secure(false),
                ))
                .configure(config),
        )
        .await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/api/auth").to_request()).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(test::read_body(response).await, r#""Unauthorized""#);
    }

    /// STATEMENT: Registration rejects a malformed invitation UUID before accessing PostgreSQL.
    ///
    /// HOW:
    /// - Initialize the real API routes and submit a password with an invalid invitation ID.
    /// - Check for `400`, the JSON content type, and the `Invalid UUID` response body.
    #[ntex::test]
    async fn invalid_invitation_uuid_is_rejected() {
        let app = test::init_service_st(
            state(),
            App::new()
                .middleware(IdentityService::new(
                    CookieIdentityPolicy::new(&[0; 32]).secure(false),
                ))
                .configure(config),
        )
        .await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/api/register/not-a-uuid")
                .json(&json!({"password": "not-used"}))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(test::read_body(response).await, r#""Invalid UUID""#);
    }

    /// STATEMENT: Logging out without an active identity is successful and returns an empty body.
    ///
    /// HOW:
    /// - Initialize the real API routes with identity middleware and no database connections.
    /// - Request `DELETE /api/auth` without a cookie and check for `200` and an empty body.
    #[ntex::test]
    async fn logout_without_identity_succeeds() {
        let app = test::init_service_st(
            state(),
            App::new()
                .middleware(IdentityService::new(
                    CookieIdentityPolicy::new(&[0; 32]).secure(false),
                ))
                .configure(config),
        )
        .await;
        let response =
            test::call_service(&app, TestRequest::delete().uri("/api/auth").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(test::read_body(response).await.is_empty());
    }
}
