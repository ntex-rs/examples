//! Remember and forget a user identity with cookie middleware.
//!
//! Look at `IdentityService` and `CookieIdentityPolicy` for configuration,
//! and the handlers for `Identity::remember` and `Identity::forget`.
//! Run with `cargo run` and open <http://127.0.0.1:8080>.

use ntex::web::{self, App, HttpResponse, middleware};
use ntex_identity::{CookieIdentityPolicy, Identity, IdentityService};

async fn index(id: Identity) -> String {
    format!(
        "Hello {}",
        id.identity().unwrap_or_else(|| "Anonymous".to_owned())
    )
}

async fn login(id: Identity) -> HttpResponse {
    id.remember("user1".to_owned());
    HttpResponse::Found().header("location", "/").build()
}

async fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Found().header("location", "/").build()
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
        .middleware(IdentityService::new(
            CookieIdentityPolicy::new(&[0; 32])
                .name("auth-example")
                .secure(false),
        ))
        // enable logger
        .middleware(middleware::Logger::default())
        .service((
            web::resource("/login").route(web::post().to(login)),
            web::resource("/logout").to(logout),
            web::resource("/").route(web::get().to(index)),
        ))
        .build()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

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

    /// STATEMENT: A login cookie identifies the user and logout removes that identity.
    ///
    /// HOW:
    /// - Check the anonymous greeting, then `POST /login` and check its redirect and `Set-Cookie`.
    /// - Send the cookie to `/` and check the authenticated greeting.
    /// - Log out with the cookie, check its removal header, and replay the cleared cookie.
    #[ntex::test]
    async fn login_cookie_is_recognized_and_logout_clears_identity() {
        let app = test::init_service(app()).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(test::read_body(response).await, "Hello Anonymous");
        let response =
            test::call_service(&app, TestRequest::post().uri("/login").to_request()).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/");
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        assert!(cookie.starts_with("auth-example="));
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/")
                .header(header::COOKIE, cookie.clone())
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(test::read_body(response).await, "Hello user1");
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/logout")
                .header(header::COOKIE, cookie)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/");
        let cleared = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cleared.contains("Max-Age=0"));
        let cookie = cleared.split(';').next().unwrap().to_owned();
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/")
                .header(header::COOKIE, cookie)
                .to_request(),
        )
        .await;
        assert_eq!(test::read_body(response).await, "Hello Anonymous");
    }

    /// STATEMENT: Untrusted cookies do not authenticate a visitor.
    ///
    /// HOW:
    /// - Send a forged `auth-example` cookie to the real app and check the anonymous greeting.
    #[ntex::test]
    async fn forged_cookie_is_anonymous() {
        let app = test::init_service(app()).await;
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/")
                .header(header::COOKIE, "auth-example=user1")
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(test::read_body(response).await, "Hello Anonymous");
    }
}
