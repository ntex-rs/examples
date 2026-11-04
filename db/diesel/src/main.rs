//! Create and look up SQLite users with Diesel and ntex.
//!
//! The handlers share an `r2d2` pool through `AppState` and run the synchronous
//! queries in `src/actions.rs` through `web::block` rather than blocking workers.
//! Apply the migrations and run from this directory as described in the README.

#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use ntex::web::{self, App, HttpResponse, WebError, middleware, types};
use uuid::Uuid;

mod actions;
mod models;
mod schema;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
type AppState = web::AppState<DbPool>;
type Error = WebError<AppState>;

/// Finds user by UID.
async fn get_user(
    pool: &AppState,
    _: (),
    user_uid: types::Path<Uuid>,
) -> Result<HttpResponse, Error> {
    let user_uid = user_uid.into_inner();
    let pool = pool.st().clone();

    // Run the synchronous Diesel query on the blocking thread pool.
    let user = web::block(move || {
        let mut conn = pool.get().expect("couldn't get db connection from pool");
        actions::find_user_by_uid(user_uid, &mut conn)
    })
    .await
    .map_err(WebError::from_err)?;

    if let Some(user) = user {
        Ok(HttpResponse::Ok().json(&user))
    } else {
        let res = HttpResponse::NotFound().body(format!("No user found with uid: {}", user_uid));
        Ok(res)
    }
}

/// Inserts a user with the name supplied in the JSON body.
async fn add_user(
    pool: &AppState,
    _: (),
    form: types::Json<models::NewUser>,
) -> Result<HttpResponse, Error> {
    let pool = pool.st().clone();
    let form = form.into_inner();

    // Run the synchronous Diesel query on the blocking thread pool.
    let user = web::block(move || {
        let mut conn = pool.get().expect("couldn't get db connection from pool");
        actions::insert_new_user(&form.name, &mut conn)
    })
    .await
    .map_err(WebError::from_err)?;

    Ok(HttpResponse::Ok().json(&user))
}

fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.route("/user/{user_id}", web::get().to_with_state(get_user))
        .route("/user", web::post().to_with_state(add_user));
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv::dotenv().ok();

    // Create the database connection pool.
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let bind = "127.0.0.1:8080";

    println!("Starting server at: {bind}");

    // Start HTTP server
    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(config)
            .build_with(AppState::new(pool.clone()))
    })
    .bind(bind, ntex::SharedCfg::new("DIESEL"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};
    use serde_json::json;

    fn memory_pool() -> DbPool {
        let pool = DbPool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(":memory:"))
            .unwrap();
        pool.get()
            .unwrap()
            .batch_execute(include_str!(
                "../migrations/20170124012402_create_users/up.sql"
            ))
            .unwrap();
        pool
    }

    /// STATEMENT: A user created through the API can be retrieved by the returned UUID.
    ///
    /// HOW:
    /// - Initialize the real routes with a single-connection in-memory SQLite pool and migration.
    /// - Post a name, check the successful JSON response and UUID, then retrieve the same user.
    #[ntex::test]
    async fn created_user_can_be_retrieved() {
        let app =
            test::init_service_st(AppState::new(memory_pool()), App::new().configure(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/user")
                .json(&json!({"name": "Ferris"}))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let created: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert_eq!(created["name"], "Ferris");
        let id = created["id"].as_str().unwrap();
        assert!(Uuid::parse_str(id).is_ok());
        let response = test::call_service(
            &app,
            TestRequest::get().uri(&format!("/user/{id}")).to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let retrieved: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert_eq!(retrieved, created);
    }

    /// STATEMENT: A valid UUID with no matching user returns the example's not-found message.
    ///
    /// HOW:
    /// - Initialize the real routes with an empty migrated in-memory database.
    /// - Request a nil UUID and check `404` and the message containing that UUID.
    #[ntex::test]
    async fn missing_user_returns_404() {
        let app =
            test::init_service_st(AppState::new(memory_pool()), App::new().configure(config)).await;
        let id = Uuid::nil();
        let response = test::call_service(
            &app,
            TestRequest::get().uri(&format!("/user/{id}")).to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            test::read_body(response).await,
            format!("No user found with uid: {id}")
        );
    }

    /// STATEMENT: Invalid UUID paths are rejected rather than queried as user IDs.
    ///
    /// HOW:
    /// - Initialize the real routes with an empty migrated in-memory database.
    /// - Request `GET /user/not-a-uuid` and check for a `404` path-extraction response.
    #[ntex::test]
    async fn invalid_user_uuid_is_rejected() {
        let app =
            test::init_service_st(AppState::new(memory_pool()), App::new().configure(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::get().uri("/user/not-a-uuid").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
