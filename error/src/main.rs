//! Custom error handling with an application-wide error domain.
//!
//! Handlers return `Result<T, E>`. When the result is an error, ntex renders
//! it with the error's [`WebResponseError`] implementation. The trait is generic
//! over two types: the application state (`St`) and the *error domain*
//! (`Err`), which is selected by the state's [`web::State::Error`] type.
//!
//! Most applications use the default domain, [`web::DefaultError`], which comes
//! with ready-made renderers for all ntex errors. This example defines its own
//! domain instead. As a result, every error that can reach a client must have a
//! `WebResponseError` implementation for that domain, including ntex's own
//! errors, such as the JSON extractor's [`JsonPayloadError`]. This is checked at
//! compile time and gives the application full control over how errors look:
//! here all of them are rendered as JSON documents with the same shape.
//!
//! See the guide for details: <https://ntex.rs/docs/errors/>.
use std::fmt;

use ntex::http::StatusCode;
use ntex::web::error::JsonPayloadError;
use ntex::web::{self, App, HttpResponse, HttpResponseBuilder, WebResponseError, types::Json};
use rand::{Rng, RngExt, distr::Distribution, distr::StandardUniform};

/// Marker type that names the application's error domain.
///
/// It is never constructed; it only appears as a type parameter of
/// `WebResponseError<AppState, AppErrors>`.
#[derive(Debug)]
struct AppErrors;

/// Application state.
///
/// The state is passed to every `error_response()` call, so renderers can use
/// it, here to include the service name in error documents. Serving an
/// application requires the state to be `Clone`.
#[derive(Clone, Debug)]
struct AppState {
    service: &'static str,
}

/// Connects the state to the error domain.
impl web::State for AppState {
    type Error = AppErrors;
}

/// The JSON document returned for every error.
#[derive(serde::Serialize)]
struct ErrorBody<'a> {
    service: &'a str,
    status: u16,
    error: String,
}

/// Renders an error document with the given status and message.
fn render_error(st: &AppState, status: StatusCode, msg: impl fmt::Display) -> HttpResponse {
    HttpResponseBuilder::new(status).json(&ErrorBody {
        service: st.service,
        status: status.as_u16(),
        error: msg.to_string(),
    })
}

/// Errors produced by the application's business logic.
///
/// `thiserror` derives `std::error::Error` and `Display`, which is required by
/// `WebResponseError`.
#[derive(Debug, thiserror::Error)]
pub enum CustomError {
    #[error("access to this resource is denied")]
    AccessDenied,
    #[error("authentication is required")]
    NotAuthenticated,
    #[error("storage backend is not reachable")]
    ServiceFailure,
    #[error("request could not be processed")]
    InvalidInput,
}

/// Lets `rand` pick a random `CustomError` variant.
impl Distribution<CustomError> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CustomError {
        match rng.random_range(0..4) {
            0 => CustomError::AccessDenied,
            1 => CustomError::NotAuthenticated,
            2 => CustomError::ServiceFailure,
            _ => CustomError::InvalidInput,
        }
    }
}

/// Converts `CustomError` into an HTTP response.
///
/// Without an `error_response()` override, every error would be rendered as
/// `500 Internal Server Error`.
impl WebResponseError<AppState, AppErrors> for CustomError {
    fn error_response(&self, st: &AppState) -> HttpResponse {
        // Rendering is also a convenient place for side effects such as
        // logging or metrics.
        log::warn!("request failed: {self}");

        match self {
            CustomError::AccessDenied => render_error(st, StatusCode::FORBIDDEN, self),
            CustomError::NotAuthenticated => render_error(st, StatusCode::UNAUTHORIZED, self),
            CustomError::InvalidInput => render_error(st, StatusCode::BAD_REQUEST, self),
            // Internal failures are not meant for users: the details stay in
            // the log and the client only gets a generic message.
            CustomError::ServiceFailure => render_error(
                st,
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ),
        }
    }
}

/// Renders errors of the `Json<T>` extractor.
///
/// ntex implements `WebResponseError` for its errors in the default domain
/// only. With a custom domain the application must provide the renderer, and
/// forgetting it is a compile error at the `.to(do_something)` registration.
impl WebResponseError<AppState, AppErrors> for JsonPayloadError {
    fn error_response(&self, st: &AppState) -> HttpResponse {
        let status = match self {
            JsonPayloadError::Overflow => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::BAD_REQUEST,
        };
        render_error(st, status, self)
    }
}

/// A container for any error of the application's domain.
///
/// A handler has a single error type, but its body may fail with several
/// different errors. Boxing them as `dyn WebResponseError` keeps the renderer
/// of the original error, and the `From` impls below let the `?` operator do
/// the conversion. (ntex's [`web::WebError`] is a similar container that is
/// built explicitly with `WebError::from_err()`.)
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct AppError(Box<dyn WebResponseError<AppState, AppErrors>>);

/// Renders the container by delegating to the boxed error.
impl WebResponseError<AppState, AppErrors> for AppError {
    fn error_response(&self, st: &AppState) -> HttpResponse {
        self.0.error_response(st)
    }
}

impl From<CustomError> for AppError {
    fn from(err: CustomError) -> Self {
        AppError(Box::new(err))
    }
}

/// Simulates business logic that fails four times out of five with a random
/// `CustomError`.
async fn do_something_random() -> Result<(), CustomError> {
    let mut rng = rand::rng();

    if rng.random_bool(0.2) {
        Ok(())
    } else {
        Err(rand::random::<CustomError>())
    }
}

#[derive(serde::Deserialize)]
struct TestPayload {
    #[allow(dead_code)]
    dummy: u32,
}

/// `POST /something` handler.
///
/// The request body is extracted with `Json<TestPayload>` before the handler
/// runs; an invalid body is rendered by the `JsonPayloadError` impl above and
/// the handler is not called. Errors from the handler body are converted to
/// `AppError` by `?`.
async fn do_something(_: Json<TestPayload>) -> Result<HttpResponse, AppError> {
    do_something_random().await?;

    Ok(HttpResponse::Ok().body("Nothing interesting happened. Try again."))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::new().default_filter_or("ntex=info,error_handling=info"),
    );

    web::server(async move |_| {
        App::new()
            // Log every request together with the response status.
            .middleware(web::middleware::Logger::default())
            .service(web::resource("/something").route(web::post().to(do_something)))
            // `build_with()` sets the application state and, through
            // `AppState::Error`, the error domain.
            .build_with(AppState {
                service: "error-example",
            })
    })
    .bind("127.0.0.1:8088", ntex::SharedCfg::new("ERROR"))?
    .run()
    .await
}
