//! Keep a visit counter in a signed cookie session.
//!
//! Look at `CookieSession` for middleware configuration and `Session`
//! for reading and updating the counter. Subsequent visits log its old value;
//! the response body remains `welcome!`. The fixed key is for local demos.
//! Run with `cargo run` and open <http://127.0.0.1:8080>.

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

/// Builds the example application, including its middleware and routes.
fn app() -> impl ntex::ServiceFactory<
    (),
    ntex::http::Request,
    Res = ntex::http::Response,
    Error = web::WebError,
    InitError = ntex::error::Failure,
> {
    App::new()
        // enable logger
        .middleware(Logger::default())
        // cookie session middleware
        .middleware(CookieSession::signed(&[0; 32]).secure(false))
        .service(index)
        .build()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    println!("Starting http server: 127.0.0.1:8080");

    web::server(async |_| app())
        .bind("127.0.0.1:8080", ntex::SharedCfg::new("COOKIE"))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};

    /// STATEMENT: Returning a signed session cookie advances the visit counter.
    ///
    /// HOW:
    /// - Request `/` and check `200`, the welcome body, and `Set-Cookie`.
    /// - Send the cookie back and check the updated cookie differs from the first.
    /// - Request without a cookie again and check the new session matches the first visit.
    #[ntex::test]
    async fn session_cookie_is_recognized_on_the_next_visit() {
        let app = test::init_service(app()).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let first = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        assert_eq!(test::read_body(response).await, "welcome!");
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/")
                .header(header::COOKIE, first.clone())
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let second = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        assert_ne!(first, second);
        assert_eq!(test::read_body(response).await, "welcome!");
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        let fresh = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        assert_eq!(fresh, first);
    }

    /// STATEMENT: Unknown routes return a routing error instead of the welcome message.
    ///
    /// HOW:
    /// - Request `/missing` through the real app and check `404` and an empty body.
    #[ntex::test]
    async fn unknown_route_is_not_found() {
        let app = test::init_service(app()).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }
}
