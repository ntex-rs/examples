//! Example of cookie based session
//! Session data is stored in cookie, it is limited to 4kb
//!
//! [Redis session example](https://github.com/ntex-rs/examples/tree/master/redis-session)

use ntex::web::{self, middleware::Logger, App, Error, HttpRequest};
use ntex_session::{CookieSession, Session};

/// simple index handler with session
#[web::get("/")]
async fn index(session: Session, req: HttpRequest) -> Result<&'static str, Error> {
    println!("{:?}", req);

    // RequestSession trait is used for session access
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
        session.set("counter", counter)?;
    } else {
        session.set("counter", counter)?;
    }

    Ok("welcome!")
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    println!("Starting http server: 127.0.0.1:8080");

    web::server(async || {
        App::new()
            // enable logger
            .middleware(Logger::default())
            // cookie session middleware
            .middleware(CookieSession::signed(&[0; 32]).secure(false))
            .service(index)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
