//! Run the ntex web basics example on the Tokio runtime.
//!
//! `#[tokio::main]` hosts `ntex::rt::System::run_local`, with routing,
//! cookie sessions, static files, streaming responses, and `InternalError`.
//! Static files are resolved relative to the crate manifest directory.

use std::{env, io};

use ntex::http::{Method, StatusCode, header};
use ntex::web::{self, App, HttpRequest, HttpResponse, error, guard, middleware};
use ntex::{channel::mpsc, util::Bytes};
use ntex_files as fs;
use ntex_session::{CookieSession, Session};

const STATIC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static");

/// favicon handler
#[web::get("/favicon")]
async fn favicon() -> Result<fs::NamedFile, io::Error> {
    fs::NamedFile::open(concat!(env!("CARGO_MANIFEST_DIR"), "/static/favicon.ico"))
}

/// simple index handler
#[web::get("/welcome")]
async fn welcome(session: Session, req: HttpRequest) -> Result<HttpResponse, error::JsonError> {
    println!("{:?}", req);

    // session
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
    }

    // set counter to session
    session.set("counter", counter)?;

    // response
    Ok(HttpResponse::builder(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/welcome.html")))
}

/// 404 handler
async fn p404() -> Result<fs::NamedFile, io::Error> {
    Ok(
        fs::NamedFile::open(concat!(env!("CARGO_MANIFEST_DIR"), "/static/404.html"))?
            .set_status_code(StatusCode::NOT_FOUND),
    )
}

/// response body
async fn response_body(path: web::types::Path<String>) -> HttpResponse {
    let text = format!("Hello {}!", *path);

    let (tx, rx_body) = mpsc::channel();
    let _ = tx.send(Ok::<_, io::Error>(Bytes::from(text)));

    HttpResponse::Ok().streaming(rx_body)
}

/// handler with path parameters like `/user/{name}/`
async fn with_param(req: HttpRequest, path: web::types::Path<(String,)>) -> HttpResponse {
    println!("{:?}", req);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Hello {}!", path.0))
}

/// Handler to match all paths starting with /files
#[web::get("/files/{all}*")]
async fn match_all_paths(path: web::types::Path<String>) -> HttpResponse {
    println!("path: {:?}", path);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("it's matching !")
}

fn app() -> impl ntex::ServiceFactory<
    (),
    ntex::http::Request,
    Res = ntex::http::Response,
    Error = web::WebError<(), web::DefaultError>,
    InitError = ntex::error::Failure,
> {
    App::new()
        // cookie session middleware
        .middleware(CookieSession::signed(&[0; 32]).secure(false))
        // enable logger
        .middleware(middleware::Logger::default())
        .service((
            // register favicon
            favicon,
            // register simple route, handle all methods
            welcome,
            // register match_all_paths method
            match_all_paths,
            // with path parameters
            web::resource("/user/{name}").route(web::get().to(with_param)),
            // async response body
            web::resource("/async-body/{name}").route(web::get().to(response_body)),
            web::resource("/test").to(|req: HttpRequest| async move {
                match *req.method() {
                    Method::GET => HttpResponse::Ok(),
                    Method::POST => HttpResponse::MethodNotAllowed(),
                    _ => HttpResponse::NotFound(),
                }
            }),
            web::resource("/error").to(|| async {
                error::InternalError::new(
                    io::Error::other("test"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }),
            // static files
            fs::Files::new("/static", STATIC_DIR).show_files_listing(),
            // redirect
            web::resource("/").route(web::get().to(|req: HttpRequest| async move {
                println!("{:?}", req);
                HttpResponse::Found()
                    .header(header::LOCATION, "static/welcome.html")
                    .build()
            })),
        ))
        // default
        .default_service(
            // 404 for GET request
            web::resource("")
                .route(web::get().to(p404))
                // all requests that are not `GET`
                .route(
                    web::route()
                        .guard(guard::Not(guard::Get()))
                        .to(|| async { HttpResponse::MethodNotAllowed() }),
                ),
        )
        .build()
}

#[tokio::main]
async fn main() -> io::Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "ntex=info");
    }
    env_logger::init();

    ntex::rt::System::new("main", ntex::rt::DefaultRuntime)
        .run_local(async {
            web::server(async |_| app())
                .bind("127.0.0.1:8080", ntex::SharedCfg::new("TOKIO"))?
                .run()
                .await
        })
        .await
}

#[cfg(test)]
mod tests {
    use ntex::web::test::{self, TestRequest};

    use super::*;

    /// STATEMENT: The welcome route serves HTML and updates its cookie session.
    ///
    /// HOW:
    /// - Initialize the real app and request `/welcome` twice, replaying the first cookie.
    /// - Check the HTML status, content type and body, and confirm the session cookie changes.
    #[ntex::test]
    async fn welcome_serves_html_and_updates_session() {
        let app = test::init_service(app()).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/welcome").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
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
        assert_eq!(
            test::read_body(response).await,
            include_str!("../static/welcome.html")
        );

        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/welcome")
                .header(header::COOKIE, cookie.as_str())
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let updated = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        assert_ne!(updated, cookie);
        assert_eq!(
            test::read_body(response).await,
            include_str!("../static/welcome.html")
        );
    }

    /// STATEMENT: The root route redirects to the static welcome page.
    ///
    /// HOW:
    /// - Send `GET /` to the real app without opening a socket.
    /// - Check `302 Found`, the `Location` header and the empty response body.
    #[ntex::test]
    async fn root_redirects_to_static_welcome() {
        let app = test::init_service(app()).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "static/welcome.html"
        );
        assert!(test::read_body(response).await.is_empty());
    }

    /// STATEMENT: Path routes extract names, stream greetings and accept nested file paths.
    ///
    /// HOW:
    /// - Request the parameter, streaming and catch-all routes on the real app.
    /// - Check successful statuses, the plain-text header where configured, and response bodies.
    #[ntex::test]
    async fn path_routes_return_expected_bodies() {
        let app = test::init_service(app()).await;
        for (path, body) in [
            ("/user/bob", "Hello bob!"),
            ("/async-body/alice", "Hello alice!"),
            ("/files/a/b/c", "it's matching !"),
        ] {
            let response =
                test::call_service(&app, TestRequest::get().uri(path).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK);
            if !path.starts_with("/async-body/") {
                assert_eq!(
                    response.headers().get(header::CONTENT_TYPE).unwrap(),
                    "text/plain"
                );
            }
            assert_eq!(test::read_body(response).await, body);
        }
    }

    /// STATEMENT: Static and fallback pages resolve independently of the working directory.
    ///
    /// HOW:
    /// - Request the static welcome page, favicon and an unknown route on the real app.
    /// - Check their statuses, file contents and the static page's HTML content type.
    #[ntex::test]
    async fn static_files_and_fallback_serve_crate_files() {
        let app = test::init_service(app()).await;
        let response = test::call_service(
            &app,
            TestRequest::get().uri("/static/welcome.html").to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert_eq!(
            test::read_body(response).await,
            include_str!("../static/welcome.html")
        );

        let response =
            test::call_service(&app, TestRequest::get().uri("/favicon").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            test::read_body(response).await.as_ref(),
            include_bytes!("../static/favicon.ico")
        );

        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            test::read_body(response).await,
            include_str!("../static/404.html")
        );
    }

    /// STATEMENT: Explicit errors and unsupported methods keep their HTTP error contracts.
    ///
    /// HOW:
    /// - Call `/error`, the three `/test` method branches and a non-`GET` fallback.
    /// - Check the expected statuses and `test` or empty response bodies.
    #[ntex::test]
    async fn errors_and_methods_return_expected_statuses() {
        let app = test::init_service(app()).await;
        for (path, method, status, body) in [
            (
                "/error",
                Method::GET,
                StatusCode::INTERNAL_SERVER_ERROR,
                "test",
            ),
            ("/test", Method::GET, StatusCode::OK, ""),
            ("/test", Method::POST, StatusCode::METHOD_NOT_ALLOWED, ""),
            ("/test", Method::DELETE, StatusCode::NOT_FOUND, ""),
            ("/missing", Method::POST, StatusCode::METHOD_NOT_ALLOWED, ""),
        ] {
            let response = test::call_service(
                &app,
                TestRequest::default().uri(path).method(method).to_request(),
            )
            .await;
            assert_eq!(response.status(), status);
            assert_eq!(test::read_body(response).await, body);
        }
    }
}
