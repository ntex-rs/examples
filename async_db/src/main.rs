/* Ntex Asynchronous Database Example

This project illustrates expensive and blocking database requests that runs
in a thread-pool using `web::block` with two examples:

    1. An asynchronous handler that executes four queries in *sequential order*,
       collecting the results and returning them as a single serialized JSON object.

    2. An asynchronous handler that executes four queries in *parallel*,
       collecting the results and returning them as a single serialized JSON object.

    Note: The use of sleep(Duration::from_secs(2)); in db.rs is to make performance
          improvement with parallelism more obvious.
 */
use std::io;

use futures::future::join_all;
use ntex::web::{self, App, AppState, HttpResponse, HttpServer, middleware};
use r2d2_sqlite::{self, SqliteConnectionManager};

mod db;
use db::{Error, Pool, Queries};

/// Runs four database queries sequentially.
async fn asyncio_weather(st: &AppState<Pool>, _: ()) -> Result<HttpResponse, Error> {
    let result = vec![
        db::execute(st, Queries::GetTopTenHottestYears).await?,
        db::execute(st, Queries::GetTopTenColdestYears).await?,
        db::execute(st, Queries::GetTopTenHottestMonths).await?,
        db::execute(st, Queries::GetTopTenColdestMonths).await?,
    ];

    Ok(HttpResponse::Ok().json(&result))
}

/// Runs four database queries concurrently.
async fn parallel_weather(st: &AppState<Pool>, _: ()) -> Result<HttpResponse, Error> {
    let fut_result = vec![
        Box::pin(db::execute(st, Queries::GetTopTenHottestYears)),
        Box::pin(db::execute(st, Queries::GetTopTenColdestYears)),
        Box::pin(db::execute(st, Queries::GetTopTenHottestMonths)),
        Box::pin(db::execute(st, Queries::GetTopTenColdestMonths)),
    ];
    let result: Result<Vec<_>, _> = join_all(fut_result).await.into_iter().collect();

    Ok(HttpResponse::Ok().json(&result?))
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let _ = env_logger::try_init();

    // Create the SQLite connection pool.
    let manager = SqliteConnectionManager::file("weather.db");
    let pool = Pool::new(manager).unwrap();

    // Start the HTTP server.
    HttpServer::new(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .route(
                "/asyncio_weather",
                web::get().to_with_state(asyncio_weather),
            )
            .route(
                "/parallel_weather",
                web::get().to_with_state(parallel_weather),
            )
            // Store the database pool in application state.
            .build_with(AppState::new(pool.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("DB"))?
    .run()
    .await
}
