//! ntex Diesel integration example.
//!
//! Diesel operations are synchronous, so this example runs them through
//! `web::block` instead of blocking a server worker.

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
        let conn = pool.get().expect("couldn't get db connection from pool");
        actions::find_user_by_uid(user_uid, &conn)
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

/// Inserts new user with name defined in form.
async fn add_user(
    pool: &AppState,
    _: (),
    form: types::Json<models::NewUser>,
) -> Result<HttpResponse, Error> {
    let pool = pool.st().clone();
    let form = form.into_inner();

    // Run the synchronous Diesel query on the blocking thread pool.
    let user = web::block(move || {
        let conn = pool.get().expect("couldn't get db connection from pool");
        actions::insert_new_user(&form.name, &conn)
    })
    .await
    .map_err(WebError::from_err)?;

    Ok(HttpResponse::Ok().json(&user))
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

    println!("Starting server at: {}", &bind);

    // Start HTTP server
    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .route("/user/{user_id}", web::get().to_with_state(get_user))
            .route("/user", web::post().to_with_state(add_user))
            .build_with(AppState::new(pool.clone()))
    })
    .bind(&bind, ntex::SharedCfg::new("DIESEL"))?
    .run()
    .await
}
