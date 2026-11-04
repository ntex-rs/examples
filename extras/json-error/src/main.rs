//! Return a custom JSON error body from a failing request handler.
//!
//! Look at the `web::WebResponseError` implementation for status selection
//! and JSON serialization. `GET /` always returns a bad-request response.
//! Run with `cargo run` and open <http://127.0.0.1:8000>.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;

use ntex::{http::StatusCode, web};
use serde::Serialize;
use serde_json::{json, to_string_pretty};

#[derive(Debug, Serialize, thiserror::Error)]
struct Error {
    msg: String,
    status: u16,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", to_string_pretty(self).unwrap())
    }
}

impl<St> web::WebResponseError<St, web::DefaultError> for Error {
    // builds the actual response to send back when an error occurs
    fn error_response(&self, _: &St) -> web::HttpResponse {
        let err_json = json!({ "error": self.msg });
        web::HttpResponse::builder(StatusCode::from_u16(self.status).unwrap()).json(&err_json)
    }
}

#[web::get("/")]
async fn index() -> Result<web::HttpResponse, Error> {
    Err(Error {
        msg: "an example error message".to_string(),
        status: 400,
    })
}

/// Registers the example's route.
fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(index);
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let ip_address = "127.0.0.1:8000";
    println!("Running server on {}", ip_address);

    web::server(async |_| web::App::new().configure(config))
        .bind(ip_address, ntex::SharedCfg::new("EX"))
        .expect("Can not bind to port 8000")
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};

    /// STATEMENT: The index renders the custom error as a JSON bad-request response.
    ///
    /// HOW:
    /// - Initialize the real route, send `GET /`, and check `400`, content type, and JSON body.
    #[ntex::test]
    async fn index_returns_custom_json_error() {
        let app = test::init_service(web::App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert_eq!(body, json!({"error": "an example error message"}));
    }

    /// STATEMENT: Unknown paths do not invoke the custom error handler.
    ///
    /// HOW:
    /// - Send `GET /missing` through the real app and check `404` and an empty body.
    #[ntex::test]
    async fn unknown_route_is_not_found() {
        let app = test::init_service(web::App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }
}
