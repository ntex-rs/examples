#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;

use std::{env, io};

use dotenv::dotenv;
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

#[ntex::main]
async fn main() -> io::Result<()> {
    dotenv().ok();

    env::set_var("RUST_LOG", "actix_todo=debug,actix_web=info");
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::init_pool(&database_url).expect("Failed to create pool");

    let app = move || {
        debug!("Constructing the App");

        let templates: Tera = Tera::new("templates/**/*").unwrap();

        let session_store = CookieSession::signed(SESSION_SIGNING_KEY).secure(false);

        web::App::new()
            .data(templates)
            .data(pool.clone())
            .wrap(Logger::default())
            .wrap(session_store)
            .service((
                web::resource("/").route(web::get().to(api::index)),
                web::resource("/todo").route(web::post().to(api::create)),
                web::resource("/todo/{id}").route(web::post().to(api::update)),
                fs::Files::new("/static", "static/"),
            ))
    };

    debug!("Starting server");
    web::server(app).bind("localhost:8088")?.run().await
}
