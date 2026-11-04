//! Four example middlewares registered on one `web::App`.
//!
//! `simple::SayHi` shows the middleware life cycle, the two `Logging`
//! middlewares inspect request and response bodies, and `redirect::CheckLogin`
//! redirects requests that are not logged in.
//!
//! Run with `cargo run -p middleware-example`.

// Deeply nested service types (HttpService + App + middlewares)
// exceed the default query depth limit.
#![recursion_limit = "256"]
#![allow(dead_code, clippy::type_complexity)]
use std::convert::Infallible;

use ntex::web;

mod read_request_body;
mod read_response_body;
mod redirect;
mod simple;

/// Registers the example's public routes.
fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/login")
            .to(|| async { "You are on /login. Go to src/redirect.rs to change this behavior." }),
    )
    .service(
        web::resource("/")
            .to(|| async { "Hello, middleware! Check the console where the server is run." }),
    );
}

/// Builds the example application, including its middleware and routes.
fn app(
    configure: impl FnOnce(&mut web::ServiceConfig),
) -> impl ntex::ServiceFactory<
    (),
    ntex::http::Request,
    Res = ntex::http::Response,
    Error = web::WebError,
    InitError = ntex::error::Failure,
> {
    web::App::new()
        .filter(|req: web::WebRequest<_>| async move {
            println!("Hi from start. You requested: {}", req.path());
            Ok::<_, Infallible>(req)
        })
        .middleware(simple::SayHi)
        .middleware(read_request_body::Logging)
        .middleware(read_response_body::Logging)
        .middleware(redirect::CheckLogin)
        .configure(configure)
        .build()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async |_| app(config))
        .bind("127.0.0.1:8080", ntex::SharedCfg::new("MW"))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};

    /// STATEMENT: The login guard redirects protected requests without returning the root greeting.
    ///
    /// HOW:
    /// - Send `GET /` through the real middleware stack and routes.
    /// - Check `302`, `Location: /login`, the outer response header, and the empty body.
    #[ntex::test]
    async fn protected_route_redirects_to_login() {
        let app = test::init_service(app(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/login");
        assert_eq!(response.headers().get("x-said-hi").unwrap(), "yes");
        assert!(test::read_body(response).await.is_empty());
    }

    /// STATEMENT: Logging middleware leaves the public login response unchanged.
    ///
    /// HOW:
    /// - Send a body to `/login` through the real app without configuring a logger.
    /// - Check `200`, the `SayHi` response header, and the complete original login message.
    #[ntex::test]
    async fn login_response_survives_logging_middleware() {
        let app = test::init_service(app(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/login")
                .payload("request body")
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-said-hi").unwrap(), "yes");
        assert_eq!(
            test::read_body(response).await,
            "You are on /login. Go to src/redirect.rs to change this behavior."
        );
    }

    /// STATEMENT: The middleware stack forwards its request header and restores the entire payload.
    ///
    /// HOW:
    /// - Add a diagnostic `/login` handler before the real routes in the shared app factory.
    /// - Send a body and check the handler sees `x-say-hi` and echoes the unchanged bytes.
    /// - Read the full response through the response-body logger and check `x-said-hi`.
    #[ntex::test]
    async fn request_header_and_payload_reach_the_handler() {
        let app = test::init_service(app(|cfg| {
            cfg.service(web::resource("/login").to(
                async |req: web::HttpRequest, body: ntex::util::Bytes| {
                    assert_eq!(req.headers().get("x-say-hi").unwrap(), "hi");
                    web::HttpResponse::Ok().body(body)
                },
            ));
            config(cfg);
        }))
        .await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/login")
                .payload("complete request payload")
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-said-hi").unwrap(), "yes");
        assert_eq!(test::read_body(response).await, "complete request payload");
    }

    /// STATEMENT: Unreadable request payloads are rejected before reaching the login handler.
    ///
    /// HOW:
    /// - Send `/login` with a payload stream yielding `PayloadError::Incomplete`.
    /// - Check the real app renders the read failure as `400`, not the successful login body.
    #[ntex::test]
    async fn request_payload_errors_are_rejected() {
        let app = test::init_service(app(config)).await;
        let mut request = TestRequest::post().uri("/login").to_request();
        *request.payload() = ntex::http::Payload::Stream(Box::pin(futures::stream::once(async {
            Err(ntex::http::error::PayloadError::Incomplete(None))
        })));
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_ne!(
            test::read_body(response).await,
            "You are on /login. Go to src/redirect.rs to change this behavior."
        );
    }
}
