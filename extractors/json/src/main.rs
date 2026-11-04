//! Compare typed JSON extraction with manual request-body parsing in ntex.
//!
//! `types::Json<MyObj>` uses a shared `JsonConfig`; `/manual` buffers a
//! `types::Payload` stream with its own limit, and `/mjsonrust` parses `Bytes`
//! with the `json` crate. See the README for request bodies and curl examples.

use futures::StreamExt;
use json::JsonValue;
use ntex::util::{Bytes, BytesMut};
use ntex::web::{self, App, HttpRequest, HttpResponse, WebError, error, middleware, types};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    name: String,
    number: i32,
}

/// Extracts JSON from the request body.
async fn index(item: types::Json<MyObj>) -> HttpResponse {
    println!("model: {:?}", item);
    HttpResponse::Ok().json(&item.0) // <- send response
}

/// Extracts JSON while applying the configured payload limit.
async fn extract_item(item: types::Json<MyObj>, req: HttpRequest) -> HttpResponse {
    println!("request: {:?}", req);
    println!("model: {:?}", item);

    HttpResponse::Ok().json(&item.0) // <- send json response
}

const MAX_SIZE: usize = 262_144; // max payload size is 256k

/// Reads the request body and deserializes the JSON manually.
async fn index_manual(mut payload: types::Payload) -> Result<HttpResponse, WebError> {
    // The payload is a stream of byte chunks.
    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(WebError::from_err)?;
        // Limit the amount of request data held in memory.
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(WebError::from_err(error::ErrorBadRequest("overflow")));
        }
        body.extend_from_slice(&chunk);
    }

    // Deserialize the buffered body with serde_json.
    let obj = serde_json::from_slice::<MyObj>(&body).map_err(WebError::from_err)?;
    Ok(HttpResponse::Ok().json(&obj)) // <- send response
}

/// Reads the request body and parses it with the json crate.
async fn index_mjsonrust(body: Bytes) -> Result<HttpResponse, WebError> {
    // Parse the buffered body with the json crate. Invalid UTF-8 and malformed
    // JSON are both reported in the `err` field.
    let result = std::str::from_utf8(&body)
        .map_err(|e| e.to_string())
        .and_then(|text| json::parse(text).map_err(|e| e.to_string()));
    let injson: JsonValue = match result {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e },
    };
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(injson.dump()))
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service((
        web::resource("/extractor").route(web::post().to(index)),
        web::resource("/extractor2").route(web::post().to(extract_item)),
        web::resource("/manual").route(web::post().to(index_manual)),
        web::resource("/mjsonrust").route(web::post().to(index_mjsonrust)),
        web::resource("/").route(web::post().to(index)),
    ));
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let cfg = ntex::SharedCfg::new("JSON")
        .add(
            web::WebAppConfig::new().set_state(types::JsonConfig::default().limit(4096)), // <- limit size of the payload (global configuration)
        )
        .build();
    let cfg2 = cfg.clone();

    web::server(async move |_| {
        App::new()
            .with_config(cfg2.get())
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(app_config)
    })
    .bind("127.0.0.1:8080", cfg)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::web::test;
    use ntex::{http, util::Bytes};

    /// STATEMENT: The JSON extractor echoes a valid typed object.
    ///
    /// HOW:
    /// - Initialize the real app and POST an object to `/`.
    /// - Check HTTP 200 and the serialized name and number in the response body.
    #[ntex::test]
    async fn test_index() -> Result<(), WebError> {
        let app = test::init_service(App::new().configure(app_config)).await;

        let req = test::TestRequest::post()
            .uri("/")
            .json(&MyObj {
                name: "my-name".to_owned(),
                number: 43,
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let bytes = test::read_body(resp).await;

        assert_eq!(bytes, Bytes::from(r##"{"name":"my-name","number":43}"##));

        Ok(())
    }

    /// STATEMENT: The `json`-crate route reports malformed JSON and invalid UTF-8 as HTTP 200 errors.
    ///
    /// HOW:
    /// - POST malformed JSON and invalid UTF-8 bytes to `/mjsonrust` in the real app.
    /// - Check HTTP 200, the JSON content type, and a single nonempty `err` string in each body.
    #[ntex::test]
    async fn mjsonrust_invalid_payloads_return_errors() {
        let app = test::init_service(App::new().configure(app_config)).await;

        for payload in [b"{notjson}".as_slice(), b"\xff".as_slice()] {
            let req = test::TestRequest::post()
                .uri("/mjsonrust")
                .header(http::header::CONTENT_TYPE, "application/json")
                .payload(payload)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), http::StatusCode::OK);
            assert_eq!(
                resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
                "application/json"
            );
            let body: serde_json::Value =
                serde_json::from_slice(&test::read_body(resp).await).unwrap();
            assert_eq!(body.as_object().unwrap().len(), 1);
            assert!(!body["err"].as_str().unwrap().is_empty());
        }
    }

    /// STATEMENT: The manual JSON route echoes a valid typed object.
    ///
    /// HOW:
    /// - POST a name and number to `/manual` using the real app configuration.
    /// - Check HTTP 200, the JSON content type, and the echoed JSON object.
    #[ntex::test]
    async fn manual_route_echoes_json() {
        let app = test::init_service(App::new().configure(app_config)).await;
        let value = serde_json::json!({"name": "John", "number": 7});
        let req = test::TestRequest::post()
            .uri("/manual")
            .json(&value)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
        assert_eq!(body, value);
    }

    /// STATEMENT: Typed JSON extraction rejects a malformed request body.
    ///
    /// HOW:
    /// - POST malformed JSON to `/extractor` in the real app.
    /// - Check that the extractor renders HTTP 400 rather than calling the handler.
    #[ntex::test]
    async fn typed_extractor_rejects_malformed_json() {
        let app = test::init_service(App::new().configure(app_config)).await;
        let req = test::TestRequest::post()
            .uri("/extractor")
            .header(http::header::CONTENT_TYPE, "application/json")
            .payload("{")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }
}
