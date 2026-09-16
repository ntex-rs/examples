//! Ntex r2d2 example
use std::io;

use ntex::web::{self, App, HttpResponse, WebError, error, middleware};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

type AppState = web::AppState<Pool<SqliteConnectionManager>>;
type Error = WebError<AppState>;

/// Async request handler. Ddb pool is stored in application state.
async fn index(
    path: web::types::Path<String>,
    db: web::types::State<AppState>,
) -> Result<HttpResponse, Error> {
    // execute sync code in threadpool
    let db = (**db).clone();
    let res = web::block(move || {
        let conn = db.get().unwrap();
        let uuid = format!("{}", uuid::Uuid::new_v4());
        conn.execute(
            "INSERT INTO users (id, name) VALUES ($1, $2)",
            &[&uuid, &path.into_inner()],
        )
        .unwrap();

        conn.query_row("SELECT name FROM users WHERE id=$1", &[&uuid], |row| {
            row.get::<_, String>(0)
        })
    })
    .await
    .map(|user| HttpResponse::Ok().json(&user))
    .map_err(|err| Error::from_err(error::ErrorInternalServerError(err)))?;
    Ok(res)
}

#[ntex::main]
async fn main() -> io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "ntex=debug");
    }
    env_logger::init();

    // r2d2 pool
    let manager = SqliteConnectionManager::file("test.db");
    let pool = r2d2::Pool::new(manager).unwrap();

    // start http server
    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .route("/{name}", web::get().to(index))
            .build_with(AppState::new(pool.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("R2D2"))?
    .run()
    .await
}
