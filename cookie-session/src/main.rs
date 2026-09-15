//! Example of cookie based session
//! Session data is stored in cookie, it is limited to 4kb
//!
//! [Redis session example](https://github.com/ntex-rs/examples/tree/master/redis-session)

use ntex::web::{self, App, HttpRequest, WebError, middleware::Logger};
use ntex_session::{CookieSession, Session};

/// simple index handler with session
#[web::get("/")]
async fn index(session: Session, req: HttpRequest) -> Result<&'static str, WebError> {
    println!("{:?}", req);

    // RequestSession trait is used for session access
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter").map_err(WebError::from_err)? {
        println!("SESSION value: {}", count);
        counter = count + 1;
        session
            .set("counter", counter)
            .map_err(WebError::from_err)?;
    } else {
        session
            .set("counter", counter)
            .map_err(WebError::from_err)?;
    }

    Ok("welcome!")
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    println!("Starting http server: 127.0.0.1:8080");

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(Logger::default())
            // cookie session middleware
            .middleware(CookieSession::signed(&[0; 32]).secure(false))
            .service(index)
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("COOKIE"))?
    .run()
    .await
}
