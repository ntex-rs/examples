// This example is meant to show how to automatically generate a json error response when something goes wrong.
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;

use ntex::http::StatusCode;
use ntex::web::{self, App, HttpRequest, WebResponseError};
use serde::Serialize;
use serde_json::{json, to_string_pretty};

#[derive(Debug, Serialize)]
struct Error {
    msg: String,
    status: u16,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", to_string_pretty(self).unwrap())
    }
}

impl WebResponseError for Error {
    // builds the actual response to send back when an error occurs
    fn error_response(&self, _: &HttpRequest) -> web::HttpResponse {
        let err_json = json!({ "error": self.msg });
        web::HttpResponse::build(StatusCode::from_u16(self.status).unwrap())
            .json(&err_json)
    }
}

#[web::get("/")]
async fn index() -> Result<web::HttpResponse, Error> {
    Err(Error {
        msg: "an example error message".to_string(),
        status: 400,
    })
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let ip_address = "127.0.0.1:8000";
    println!("Running server on {}", ip_address);

    web::server(async || App::new().service(index))
        .bind(ip_address)
        .expect("Can not bind to port 8000")
        .run()
        .await
}
