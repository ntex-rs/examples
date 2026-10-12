//! A tour of the basic building blocks of an ntex web application:
//!
//! * handlers registered with route macros (`#[web::get(..)]`) and with
//!   `web::resource()`;
//! * path parameters, including a "tail" segment that matches the rest of the path;
//! * cookie-based sessions;
//! * streaming response bodies;
//! * serving individual files and whole directories;
//! * redirects, errors and a custom default (404) service.
//!
//! See the guide for details: <https://ntex.rs/docs/handlers/>,
//! <https://ntex.rs/docs/url-dispatch/>, <https://ntex.rs/docs/static-files/>.
use std::io;

use ntex::http::{Method, StatusCode, header};
use ntex::web::{self, App, HttpRequest, HttpResponse, error, guard, middleware};
use ntex::{SharedCfg, channel::mpsc, util::Bytes};
use ntex_files as fs;
use ntex_session::{CookieSession, Session};

/// Directory with static files.
///
/// The path is resolved relative to the crate directory at compile time, so the
/// example works no matter which directory `cargo run` is started from.
const STATIC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static");

/// Serves a single file.
///
/// The `#[web::get("/favicon")]` macro turns the function into a service that
/// handles `GET /favicon`; it is registered with `App::service()` below.
/// `NamedFile` implements `Responder`: it sets `Content-Type` from the file
/// extension and supports conditional and range requests.
#[web::get("/favicon")]
async fn favicon() -> Result<fs::NamedFile, io::Error> {
    // A missing file is returned as an `io::Error`, which ntex turns
    // into an error response (404 for `NotFound`).
    fs::NamedFile::open(format!("{STATIC_DIR}/favicon.ico"))
}

/// Counts visits with a cookie session and returns an HTML page.
///
/// `Session` is an extractor provided by the `CookieSession` middleware.
/// Values are serialized to JSON and stored in a signed cookie, so a
/// (de)serialization failure is reported as `JsonError`.
#[web::get("/welcome")]
async fn welcome(session: Session) -> Result<HttpResponse, error::JsonError> {
    // Read the previous value, if this client has a session already.
    let counter = session.get::<u32>("counter")?.unwrap_or(0) + 1;

    // Store the new value. The middleware writes the updated cookie to the
    // response after the handler returns.
    session.set("counter", counter)?;

    // The page is embedded into the binary at compile time.
    let page = include_str!("../static/welcome.html").replace(
        "<!-- visits -->",
        &format!("<p>Visits in this session: {counter}</p>"),
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page))
}

/// Default handler for `GET` requests that do not match any route.
async fn p404() -> Result<fs::NamedFile, io::Error> {
    // Serve the page with a 404 status instead of the default 200.
    Ok(fs::NamedFile::open(format!("{STATIC_DIR}/404.html"))?
        .set_status_code(StatusCode::NOT_FOUND))
}

/// Streams the response body instead of building it in memory.
///
/// Any `Stream<Item = Result<Bytes, E>>` can be used as a body; here it is the
/// receiving half of a channel. The response is sent with chunked transfer
/// encoding, and it is complete when all senders are dropped.
async fn response_body(path: web::types::Path<String>) -> HttpResponse {
    let (tx, rx_body) = mpsc::channel();

    // The sender could be moved to another task to produce chunks later.
    // `tx` is dropped at the end of the function, which ends the stream.
    let _ = tx.send(Ok::<_, io::Error>(Bytes::from(format!("Hello {}!", *path))));

    HttpResponse::Ok().streaming(rx_body)
}

/// Handler with a path parameter, registered for `/user/{name}`.
///
/// `Path<T>` deserializes the dynamic segments of the matched path into `T`
/// (here a one-element tuple; a `String` or a struct with a `name` field
/// works too). If deserialization fails, the request is rejected with 404.
async fn with_param(path: web::types::Path<(String,)>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Hello {}!", path.0))
}

/// Matches every path that starts with `/files/`.
///
/// `{all}*` is a "tail" segment: unlike a regular `{name}` segment it also
/// matches `/`, so `/files/a/b/c` gives `all == "a/b/c"`.
#[web::get("/files/{all}*")]
async fn match_all_paths(path: web::types::Path<String>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("matched tail: {}\n", *path))
}

#[ntex::main]
async fn main() -> io::Result<()> {
    // Log requests by default; the RUST_LOG env variable overrides this.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("ntex=info")).init();

    // The closure is called once per worker thread to build the application.
    web::server(async |_| {
        App::new()
            // Middlewares wrap the whole application. The one registered last
            // is the outermost, so the logger sees the response produced by
            // the session middleware, including its `Set-Cookie` header.
            //
            // The session cookie is signed with a 32-byte key. Use a real
            // random key in production and `secure(true)` with HTTPS.
            .middleware(CookieSession::signed(&[0; 32]).secure(false))
            .middleware(middleware::Logger::default())
            .service((
                // Services generated by the route macros.
                favicon,
                welcome,
                match_all_paths,
                // A resource with a path parameter and a single `GET` route.
                // Other methods get "405 Method Not Allowed".
                web::resource("/user/{name}").route(web::get().to(with_param)),
                // Streaming response body.
                web::resource("/async-body/{name}").route(web::get().to(response_body)),
                // `resource().to()` accepts every method, so the handler can
                // inspect the method itself.
                web::resource("/test").to(async |req: HttpRequest| match *req.method() {
                    Method::GET => HttpResponse::Ok(),
                    Method::POST => HttpResponse::MethodNotAllowed(),
                    _ => HttpResponse::NotFound(),
                }),
                // Any type that implements `WebResponseError` can be returned
                // as an error. `InternalError` wraps an arbitrary error and
                // sends it with the given status code.
                web::resource("/error").to(async || {
                    error::InternalError::new(
                        io::Error::other("test"),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }),
                // Serve files from the `static` directory under `/static/...`.
                // With `show_files_listing()`, `GET /static/` returns an
                // HTML listing of the directory.
                fs::Files::new("/static", STATIC_DIR).show_files_listing(),
                // Redirect the root to the welcome page.
                web::resource("/").route(web::get().to(async || {
                    HttpResponse::Found()
                        .header(header::LOCATION, "/welcome")
                        .build()
                })),
            ))
            // The default service handles requests that match no route.
            // The empty resource path is ignored for the default service.
            .default_service(
                web::resource("")
                    // A custom 404 page for `GET` requests.
                    .route(web::get().to(p404))
                    // 405 for all other methods. A route can have several
                    // guards; here a single `Not` guard selects every request
                    // that is not `GET`.
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(async || HttpResponse::MethodNotAllowed()),
                    ),
            )
    })
    .bind("127.0.0.1:8080", SharedCfg::default())?
    .run()
    .await
}
