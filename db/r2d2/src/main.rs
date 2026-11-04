//! Insert and retrieve names with a SQLite connection pool in ntex.
//!
//! `AppState` shares an `r2d2` pool, `types::Path` extracts the name, and
//! `web::block` moves synchronous database operations off server workers.
//! Create the `users` table in `test.db` as described in the README, then
//! run with `cargo run` from this directory.

use std::io;

use ntex::web::{self, App, HttpResponse, WebError, error, middleware};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

type AppState = web::AppState<Pool<SqliteConnectionManager>>;
type Error = WebError<AppState>;

/// Uses the database pool stored in application state.
async fn index(
    db: &AppState,
    _: (),
    path: web::types::Path<String>,
) -> Result<HttpResponse, Error> {
    // Run the synchronous database work on the blocking thread pool.
    let db = db.st().clone();
    let res = web::block(move || {
        let conn = db.get().unwrap();
        let uuid = format!("{}", uuid::Uuid::new_v4());
        conn.execute(
            "INSERT INTO users (id, name) VALUES ($1, $2)",
            [&uuid, &path.into_inner()],
        )
        .unwrap();

        conn.query_row("SELECT name FROM users WHERE id=$1", [&uuid], |row| {
            row.get::<_, String>(0)
        })
    })
    .await
    .map(|user| HttpResponse::Ok().json(&user))
    .map_err(|err| Error::from_err(error::ErrorInternalServerError(err)))?;
    Ok(res)
}

fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.route("/{name}", web::get().to_with_state(index));
}

#[ntex::main]
async fn main() -> io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=debug");
    }
    env_logger::init();

    // Create the r2d2 connection pool.
    let manager = SqliteConnectionManager::file("test.db");
    let pool = r2d2::Pool::new(manager).unwrap();

    // Start the HTTP server.
    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(config)
            .build_with(AppState::new(pool.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("R2D2"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};

    fn memory_pool() -> Pool<SqliteConnectionManager> {
        let pool = Pool::builder()
            .max_size(1)
            .build(SqliteConnectionManager::memory())
            .unwrap();
        pool.get()
            .unwrap()
            .execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL)")
            .unwrap();
        pool
    }

    /// STATEMENT: A name from the request path is stored in SQLite and returned as JSON.
    ///
    /// HOW:
    /// - Initialize the real routes with an in-memory SQLite pool and a `users` table.
    /// - Request `GET /Ada%20Lovelace` and check the status, content type, and decoded name.
    /// - Query the retained pool to verify that the same name was inserted.
    #[ntex::test]
    async fn name_is_inserted_and_returned() {
        let pool = memory_pool();
        let app =
            test::init_service_st(AppState::new(pool.clone()), App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/Ada%20Lovelace").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(test::read_body(response).await, r#""Ada Lovelace""#);
        let name: String = pool
            .get()
            .unwrap()
            .query_row("SELECT name FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Ada Lovelace");
    }

    /// STATEMENT: An unmatched route does not insert a user.
    ///
    /// HOW:
    /// - Initialize the real routes with an empty in-memory `users` table.
    /// - Request `GET /`, check for `404`, and verify that the table remains empty.
    #[ntex::test]
    async fn unknown_route_is_404() {
        let pool = memory_pool();
        let app =
            test::init_service_st(AppState::new(pool.clone()), App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let count: i64 = pool
            .get()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
