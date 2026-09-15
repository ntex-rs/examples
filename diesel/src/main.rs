//! Ntex Diesel integration example
//!
//! Diesel does not support tokio, so we have to run it in separate threads using the web::block
//! function which offloads blocking code (like Diesel's) in order to not block the server's thread.

#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use ntex::web::{self, types, middleware, App, WebError, HttpResponse};
use uuid::Uuid;

mod actions;
mod models;
mod schema;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

/// Finds user by UID.
#[web::get("/user/{user_id}", state=web::State<DbPool>)]
async fn get_user(
    pool: types::State<web::State<DbPool>>,
    user_uid: types::Path<Uuid>,
) -> Result<HttpResponse, WebError<web::State<DbPool>>> {
    let user_uid = user_uid.into_inner();
    let conn = pool.get().expect("couldn't get db connection from pool");

    // use web::block to offload blocking Diesel code without blocking server thread
    let user = web::block(move || actions::find_user_by_uid(user_uid, &conn)).await.map_err(WebError::from_err)?;

    if let Some(user) = user {
        Ok(HttpResponse::Ok().json(&user))
    } else {
        let res = HttpResponse::NotFound()
            .body(format!("No user found with uid: {}", user_uid));
        Ok(res)
    }
}

/// Inserts new user with name defined in form.
#[web::post("/user", state=web::State<DbPool>)]
async fn add_user(
    pool: types::State<web::State<DbPool>>,
    form: types::Json<models::NewUser>,
) -> Result<HttpResponse, WebError<web::State<DbPool>>> {
    let conn = pool.get().expect("couldn't get db connection from pool");

    // use web::block to offload blocking Diesel code without blocking server thread
    let user = web::block(move || actions::insert_new_user(&form.name, &conn)).await.map_err(WebError::from_err)?;

    Ok(HttpResponse::Ok().json(&user))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv::dotenv().ok();

    // set up database connection pool
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
            .service((get_user, add_user))
            // set up DB pool to be used with web::State<Pool> extractor
            .build_with(web::State::new(pool.clone()))
    })
    .bind(&bind, ntex::SharedCfg::new("DIESEL"))?
    .run()
    .await
}
