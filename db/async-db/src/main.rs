//! Compare sequential and concurrent SQLite queries with ntex.
//!
//! `src/db.rs` runs synchronous queries through `web::block`; the handlers
//! share an `r2d2` pool through `AppState` and return JSON results.
//! An artificial two-second delay per query highlights the scheduling difference.
//! Initialize the database as described in the README and run from this directory.

use std::io;

use futures::future::join_all;
use ntex::web::{self, App, AppState, HttpResponse, HttpServer, middleware};
use r2d2_sqlite::{self, SqliteConnectionManager};

mod db;
use db::{Error, Pool, Queries};

/// Runs four database queries sequentially.
async fn asyncio_weather(st: &AppState<Pool>, _: ()) -> Result<HttpResponse, Error> {
    let result = vec![
        db::execute(st, Queries::HottestYears).await?,
        db::execute(st, Queries::ColdestYears).await?,
        db::execute(st, Queries::HottestMonths).await?,
        db::execute(st, Queries::ColdestMonths).await?,
    ];

    Ok(HttpResponse::Ok().json(&result))
}

/// Runs four database queries concurrently.
async fn parallel_weather(st: &AppState<Pool>, _: ()) -> Result<HttpResponse, Error> {
    let fut_result = vec![
        Box::pin(db::execute(st, Queries::HottestYears)),
        Box::pin(db::execute(st, Queries::ColdestYears)),
        Box::pin(db::execute(st, Queries::HottestMonths)),
        Box::pin(db::execute(st, Queries::ColdestMonths)),
    ];
    let result: Result<Vec<_>, _> = join_all(fut_result).await.into_iter().collect();

    Ok(HttpResponse::Ok().json(&result?))
}

fn config(cfg: &mut web::ServiceConfig<AppState<Pool>>) {
    cfg.route(
        "/asyncio_weather",
        web::get().to_with_state(asyncio_weather),
    )
    .route(
        "/parallel_weather",
        web::get().to_with_state(parallel_weather),
    );
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let _ = env_logger::try_init();

    // Start N db executor actors (N = number of cores avail)
    let manager = SqliteConnectionManager::file("weather.db");
    let pool = Pool::new(manager).unwrap();

    // Start the HTTP server.
    HttpServer::new(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(config)
            // Store the database pool in application state.
            .build_with(AppState::new(pool.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("DB"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};
    use serde_json::json;

    fn weather_pool() -> Pool {
        let pool = Pool::builder()
            .max_size(1)
            .build(SqliteConnectionManager::memory())
            .unwrap();
        pool.get()
            .unwrap()
            .execute_batch(
                "CREATE TABLE nyc_weather (date TEXT, tmax REAL);
                 INSERT INTO nyc_weather VALUES
                     ('2020-01-01', 10), ('2020-01-02', 20), ('2021-02-01', -5);",
            )
            .unwrap();
        pool
    }

    fn expected_weather() -> serde_json::Value {
        json!([
            [{"AnnualAgg": {"year": 2020, "total": 30.0}},
             {"AnnualAgg": {"year": 2021, "total": -5.0}}],
            [{"AnnualAgg": {"year": 2021, "total": -5.0}},
             {"AnnualAgg": {"year": 2020, "total": 30.0}}],
            [{"MonthAgg": {"year": 2020, "month": 1, "total": 30.0}},
             {"MonthAgg": {"year": 2021, "month": 2, "total": -5.0}}],
            [{"MonthAgg": {"year": 2021, "month": 2, "total": -5.0}},
             {"MonthAgg": {"year": 2020, "month": 1, "total": 30.0}}],
        ])
    }

    /// STATEMENT: Sequential weather queries return correctly ranked annual and monthly totals.
    ///
    /// HOW:
    /// - Initialize the real routes with a seeded in-memory SQLite pool and no simulated delay.
    /// - Request `GET /asyncio_weather` and check the status, JSON content type, and all aggregates.
    #[ntex::test]
    async fn sequential_weather_returns_ranked_aggregates() {
        let app =
            test::init_service_st(AppState::new(weather_pool()), App::new().configure(config))
                .await;
        let response = test::call_service(
            &app,
            TestRequest::get().uri("/asyncio_weather").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert_eq!(body, expected_weather());
    }

    /// STATEMENT: Concurrent weather queries preserve the order and results of all four queries.
    ///
    /// HOW:
    /// - Initialize the real routes with a seeded in-memory SQLite pool and no simulated delay.
    /// - Request `GET /parallel_weather` and check the status, JSON content type, and ordered totals.
    #[ntex::test]
    async fn parallel_weather_returns_ranked_aggregates() {
        let app =
            test::init_service_st(AppState::new(weather_pool()), App::new().configure(config))
                .await;
        let response = test::call_service(
            &app,
            TestRequest::get().uri("/parallel_weather").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert_eq!(body, expected_weather());
    }

    /// STATEMENT: A missing weather table produces an HTTP server error rather than a successful result.
    ///
    /// HOW:
    /// - Initialize the real routes with an empty in-memory SQLite pool and no simulated delay.
    /// - Request both weather endpoints and check for `500` and the missing-table error body.
    #[ntex::test]
    async fn missing_weather_table_returns_server_error() {
        let pool = Pool::builder()
            .max_size(1)
            .build(SqliteConnectionManager::memory())
            .unwrap();
        let app = test::init_service_st(AppState::new(pool), App::new().configure(config)).await;

        for path in ["/asyncio_weather", "/parallel_weather"] {
            let response =
                test::call_service(&app, TestRequest::get().uri(path).to_request()).await;
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            let body = test::read_body(response).await;
            assert!(
                std::str::from_utf8(&body)
                    .unwrap()
                    .contains("no such table: nyc_weather")
            );
        }
    }
}
