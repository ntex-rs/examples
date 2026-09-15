// This is a contrived example intended to illustrate ntex features.
// *Imagine* that you have a process that involves 3 steps.  The steps here
// are dumb in that they do nothing other than call an
// httpbin endpoint that returns the json that was posted to it.  The intent
// here is to illustrate how to chain these steps together as futures and return
// a final result in a response.
//
// Ntex features illustrated here include:
//     1. handling json input param
//     2. validating user-submitted parameters using the 'validator' crate
//     2. ntex client features:
//           - POSTing json body
//     3. chaining futures into a single response used by an async endpoint
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

/// validate data, post json to httpbin, get it back in the response body, return deserialized
async fn step_x(data: SomeData, client: &Client) -> Result<SomeData, Error> {
    // validate data
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

#[web::post("/something", state=AppState<Client>)]
async fn create_something(
    some_data: types::Json<SomeData>,
    st: types::State<AppState<Client>>,
) -> Result<HttpResponse, Error> {
    let some_data_2 = step_x(some_data.into_inner(), &st).await?;
    let some_data_3 = step_x(some_data_2, &st).await?;
    let d = step_x(some_data_3, &st).await?;

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
            .service(create_something)
            .build_with(AppState::new(Client::new()))
    })
    .bind(endpoint, SharedCfg::new("EX1"))?
    .run()
    .await
}
