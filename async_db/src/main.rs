/* Ntex Asynchronous Database Example

This project illustrates expensive and blocking database requests that runs
in a thread-pool using `web::block` with two examples:

    1. An asynchronous handler that executes 4 queries in *sequential order*,
       collecting the results and returning them as a single serialized json object

    2. An asynchronous handler that executes 4 queries in *parallel*,
       collecting the results and returning them as a single serialized json object

    Note: The use of sleep(Duration::from_secs(2)); in db.rs is to make performance
          improvement with parallelism more obvious.
 */
use std::io;

use futures::future::join_all;
use ntex::web::{self, middleware, App, HttpResponse, HttpServer};
use r2d2_sqlite::{self, SqliteConnectionManager};

mod db;
use db::{Error, Pool, Queries};

/// Version 1: Calls 4 queries in sequential order, as an asynchronous handler
#[web::get("/asyncio_weather")]
async fn asyncio_weather(db: web::types::State<Pool>) -> Result<HttpResponse, Error> {
    let result = vec![
        db::execute(&db, Queries::GetTopTenHottestYears).await?,
        db::execute(&db, Queries::GetTopTenColdestYears).await?,
        db::execute(&db, Queries::GetTopTenHottestMonths).await?,
        db::execute(&db, Queries::GetTopTenColdestMonths).await?,
    ];

    Ok(HttpResponse::Ok().json(&result))
}

/// Version 2: Calls 4 queries in parallel, as an asynchronous handler
/// Returning Error types turn into None values in the response
#[web::get("/parallel_weather")]
async fn parallel_weather(db: web::types::State<Pool>) -> Result<HttpResponse, Error> {
    let fut_result = vec![
        Box::pin(db::execute(&db, Queries::GetTopTenHottestYears)),
        Box::pin(db::execute(&db, Queries::GetTopTenColdestYears)),
        Box::pin(db::execute(&db, Queries::GetTopTenHottestMonths)),
        Box::pin(db::execute(&db, Queries::GetTopTenColdestMonths)),
    ];
    let result: Result<Vec<_>, _> = join_all(fut_result).await.into_iter().collect();

    Ok(HttpResponse::Ok().json(&result?))
}

#[ntex::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=info");
    env_logger::init();

    // Start N db executor actors (N = number of cores avail)
    let manager = SqliteConnectionManager::file("weather.db");
    let pool = Pool::new(manager).unwrap();

    // Start http server
    HttpServer::new(async move || {
        App::new()
            // store db pool as Data object
            .state(pool.clone())
            .middleware(middleware::Logger::default())
            .service((asyncio_weather, parallel_weather))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
