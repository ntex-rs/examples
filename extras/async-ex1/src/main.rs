//! Chain validated JSON through three asynchronous HTTP requests.
//!
//! Look at `step_x` for `validator` validation and httpbin POST requests,
//! and `create_something` for sequencing them with a shared HTTP client.
//! Run with `cargo run --features ntex/openssl`; POST to `/something` on
//! <http://127.0.0.1:8080>. Internet access to httpbin is required.

use std::io;

use futures::StreamExt;
use ntex::web::{self, App, AppState, HttpResponse, error::ErrorBadRequest, types};
use ntex::{SharedCfg, client::Client, util::BytesMut};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone)]
struct Upstream {
    client: Client,
    url: String,
}

type State = AppState<Upstream>;
type Error = web::WebError<State, web::DefaultError>;

#[derive(Debug, Validate, Deserialize, Serialize)]
struct SomeData {
    #[validate(length(min = 1, max = 1000000))]
    id: String,
    #[validate(length(min = 1, max = 100))]
    name: String,
}

/// The part of httpbin's response this example reads. Serde ignores the other fields.
#[derive(Debug, Deserialize)]
struct HttpBinResponse {
    json: SomeData,
}

/// Validates the data, sends it to httpbin, and returns the echoed JSON.
async fn step_x(data: SomeData, upstream: &Upstream) -> Result<SomeData, Error> {
    // Validate the submitted data before sending it.
    data.validate()
        .map_err(|e| Error::from_err(ErrorBadRequest(e)))?;

    let mut res = upstream
        .client
        .post(upstream.url.as_str())
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
    st: &State,
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

fn app_config(config: &mut web::ServiceConfig<State>) {
    config.route("/something", web::post().to_with_state(create_something));
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let endpoint = "127.0.0.1:8080";

    println!("Starting server at: {:?}", endpoint);
    web::HttpServer::new(async |_| {
        App::new()
            .configure(app_config)
            .build_with(AppState::new(Upstream {
                client: Client::new(),
                url: "https://httpbin.org/post".to_owned(),
            }))
    })
    .bind(endpoint, SharedCfg::new("EX1"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test;

    /// STATEMENT: The route passes each upstream result into the next of three asynchronous calls.
    ///
    /// HOW:
    /// - Start a local upstream that appends `!` to the submitted name on each request.
    /// - Initialize the real app with that upstream and POST a valid object to `/something`.
    /// - Check HTTP 200, the JSON content type, the original ID, and three appended characters.
    #[ntex::test]
    async fn successful_request_chains_three_upstream_calls() {
        let upstream = test::server(async |_| {
            App::new().route(
                "/post",
                web::post().to(async |body: types::Json<SomeData>| {
                    let mut data = body.into_inner();
                    data.name.push('!');
                    HttpResponse::Ok().json(&serde_json::json!({"json": data}))
                }),
            )
        });
        let app = test::init_service_st(
            AppState::new(Upstream {
                client: Client::new(),
                url: upstream.url("/post"),
            }),
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/something")
            .json(&serde_json::json!({"id": "1", "name": "John"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
        assert_eq!(body, serde_json::json!({"id": "1", "name": "John!!!"}));
    }

    /// STATEMENT: Validation rejects invalid IDs and names without contacting the upstream.
    ///
    /// HOW:
    /// - Initialize the real app with an unusable upstream URL.
    /// - POST empty IDs, empty names, and a name over 100 characters to `/something`.
    /// - Check HTTP 400 and the invalid field name in each rendered error body.
    #[ntex::test]
    async fn invalid_data_is_bad_request() {
        let app = test::init_service_st(
            AppState::new(Upstream {
                client: Client::new(),
                url: String::new(),
            }),
            App::new().configure(app_config),
        )
        .await;
        for (id, name, field) in [
            ("", "John".to_owned(), "id"),
            ("1", String::new(), "name"),
            ("1", "a".repeat(101), "name"),
        ] {
            let req = test::TestRequest::post()
                .uri("/something")
                .json(&serde_json::json!({"id": id, "name": name}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
            let body = test::read_body(resp).await;
            assert!(std::str::from_utf8(&body).unwrap().contains(field));
        }
    }

    /// STATEMENT: The route rejects malformed JSON before contacting the upstream.
    ///
    /// HOW:
    /// - Initialize the real app with an unusable upstream URL and POST malformed JSON.
    /// - Check that `/something` returns HTTP 400.
    #[ntex::test]
    async fn malformed_json_is_bad_request() {
        let app = test::init_service_st(
            AppState::new(Upstream {
                client: Client::new(),
                url: String::new(),
            }),
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/something")
            .header(header::CONTENT_TYPE, "application/json")
            .payload("{")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
