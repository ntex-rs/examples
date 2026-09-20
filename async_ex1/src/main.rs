// This example runs submitted data through three asynchronous steps. Each step
// posts the JSON to httpbin and reads the echoed value from its response.
//
// It demonstrates JSON extraction, validation with the `validator` crate,
// outgoing HTTP requests, and chaining async operations in a handler.
use std::io;

use futures::StreamExt;
use ntex::web::{self, App, AppState, HttpResponse, error::ErrorBadRequest, types};
use ntex::{SharedCfg, client::Client, util::BytesMut, util::HashMap};
use serde::{Deserialize, Serialize};
use validator::Validate;
use validator_derive::Validate;

type State = AppState<Client>;
type Error = web::WebError<State, web::DefaultError>;

#[derive(Debug, Validate, Deserialize, Serialize)]
struct SomeData {
    #[validate(length(min = 1, max = 1000000))]
    id: String,
    #[validate(length(min = 1, max = 100))]
    name: String,
}

#[derive(Debug, Deserialize)]
struct HttpBinResponse {
    args: HashMap<String, String>,
    data: String,
    files: HashMap<String, String>,
    form: HashMap<String, String>,
    headers: HashMap<String, String>,
    json: SomeData,
    origin: String,
    url: String,
}

/// Validates the data, sends it to httpbin, and returns the echoed JSON.
async fn step_x(data: SomeData, client: &Client) -> Result<SomeData, Error> {
    // Validate the submitted data before sending it.
    data.validate()
        .map_err(|e| Error::from_err(ErrorBadRequest(e)))?;

    let mut res = client
        .post("https://httpbin.org/post")
        .send_json(&data)
        .await
        .map_err(Error::from_err)?; // <- convert ClientError to an WebError

    let mut body = BytesMut::new();
    while let Some(chunk) = res.next().await {
        body.extend_from_slice(&chunk.map_err(Error::from_err)?);
    }

    let body: HttpBinResponse = serde_json::from_slice(&body).unwrap();
    Ok(body.json)
}

async fn create_something(
    st: &AppState<Client>,
    _: (),
    some_data: types::Json<SomeData>,
) -> Result<HttpResponse, Error> {
    let some_data_2 = step_x(some_data.into_inner(), st).await?;
    let some_data_3 = step_x(some_data_2, st).await?;
    let d = step_x(some_data_3, st).await?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&d).unwrap()))
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let endpoint = "127.0.0.1:8080";

    println!("Starting server at: {:?}", endpoint);
    web::HttpServer::new(async |_| {
        App::new()
            .route("/something", web::post().to_with_state(create_something))
            .build_with(AppState::new(Client::new()))
    })
    .bind(endpoint, SharedCfg::new("EX1"))?
    .run()
    .await
}
