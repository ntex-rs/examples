//! Render a PostgreSQL-backed todo list with cookie flash messages.
//!
//! Look at `AppConfig` for worker-local templates, `api` for form handlers,
//! and `db` for blocking Diesel queries. Configure `DATABASE_URL` and apply
//! Diesel migrations before running `cargo run` from the crate directory.
//! The server listens on <http://localhost:8088>.

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;

use std::{env, io};

use dotenv::dotenv;
use ntex::server::ServerAppConfig;
use ntex::web;
use ntex::web::middleware::Logger;
use ntex_files as fs;
use ntex_session::CookieSession;
use tera::Tera;

mod api;
mod db;
mod model;
mod schema;
mod session;

static SESSION_SIGNING_KEY: &[u8] = &[0; 32];

#[derive(Clone)]
pub struct AppState {
    pool: db::PgPool,
    templates: Tera,
}

impl web::State for AppState {
    type Error = web::DefaultError;
}

struct AppConfig {
    pool: db::PgPool,
}

impl ServerAppConfig for AppConfig {
    type State = AppState;

    async fn create(&self) -> io::Result<Self::State> {
        Ok(AppState {
            pool: self.pool.clone(),
            templates: {
                let mut tera = Tera::new();
                tera.load_from_glob("templates/**/*")
                    .map_err(io::Error::other)?;
                tera
            },
        })
    }
}

fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.service((
        web::resource("/").route(web::get().to_with_state(api::index)),
        web::resource("/todo").route(web::post().to_with_state(api::create)),
        web::resource("/todo/{id}").route(web::post().to_with_state(api::update)),
    ));
}

#[ntex::main]
async fn main() -> io::Result<()> {
    dotenv().ok();

    unsafe {
        env::set_var("RUST_LOG", "todo=debug,ntex=info");
    }
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::init_pool(&database_url).expect("Failed to create pool");

    let app = async move |_: &AppState| {
        debug!("Constructing the App");

        let session_store = CookieSession::signed(SESSION_SIGNING_KEY).secure(false);

        web::App::new()
            .middleware(Logger::default())
            .middleware(session_store)
            .configure(config)
            .service(fs::Files::new("/static", "static/"))
    };

    debug!("Starting server");
    web::server_with_config(AppConfig { pool }, app)
        .bind("localhost:8088", ntex::SharedCfg::new("TODO"))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::pg::PgConnection;
    use diesel::r2d2::ConnectionManager;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};

    fn state() -> AppState {
        // Validation paths must not acquire a connection or contact PostgreSQL.
        let pool = db::PgPool::builder()
            .max_size(1)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::<PgConnection>::new(
                "postgres://localhost/unused",
            ));
        AppState {
            pool,
            templates: Tera::new(),
        }
    }

    /// STATEMENT: An empty description redirects to the index and stores a flash message without a database call.
    ///
    /// HOW:
    /// - Initialize the real routes with signed sessions and a pool that opens no connections.
    /// - Submit an empty `description` to `POST /todo`.
    /// - Check for a `302` redirect to `/`, a session cookie, and an empty response body.
    #[ntex::test]
    async fn empty_description_redirects_with_flash() {
        let app = test::init_service_st(
            state(),
            web::App::new()
                .middleware(CookieSession::signed(SESSION_SIGNING_KEY).secure(false))
                .configure(config),
        )
        .await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/todo")
                .form(&[("description", "")])
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/");
        assert!(response.headers().contains_key(header::SET_COOKIE));
        assert!(test::read_body(response).await.is_empty());
    }

    /// STATEMENT: Unsupported todo update methods return a client error without accessing PostgreSQL.
    ///
    /// HOW:
    /// - Initialize the real routes with signed sessions and a pool that opens no connections.
    /// - Submit `_method=patch` to `POST /todo/1`.
    /// - Check for `400` and the diagnostic naming the unsupported method.
    #[ntex::test]
    async fn unsupported_update_method_is_rejected() {
        let app = test::init_service_st(
            state(),
            web::App::new()
                .middleware(CookieSession::signed(SESSION_SIGNING_KEY).secure(false))
                .configure(config),
        )
        .await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/todo/1")
                .form(&[("_method", "patch")])
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            test::read_body(response).await,
            "Unsupported HTTP method: patch"
        );
    }
}
