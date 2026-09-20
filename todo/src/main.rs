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
            templates: Tera::new("templates/**/*").map_err(io::Error::other)?,
        })
    }
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
            .service((
                web::resource("/").route(web::get().to_with_state(api::index)),
                web::resource("/todo").route(web::post().to_with_state(api::create)),
                web::resource("/todo/{id}").route(web::post().to_with_state(api::update)),
                fs::Files::new("/static", "static/"),
            ))
    };

    debug!("Starting server");
    web::server_with_config(AppConfig { pool }, app)
        .bind("localhost:8088", ntex::SharedCfg::new("TODO"))?
        .run()
        .await
}
