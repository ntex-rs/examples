use futures::StreamExt;
use json::JsonValue;
use ntex::util::{Bytes, BytesMut};
use ntex::web::{self, error, middleware, App, Error, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    name: String,
    number: i32,
}

/// This handler uses json extractor
async fn index(item: web::types::Json<MyObj>) -> HttpResponse {
    println!("model: {:?}", &item);
    HttpResponse::Ok().json(&item.0) // <- send response
}

/// This handler uses json extractor with limit
async fn extract_item(item: web::types::Json<MyObj>, req: HttpRequest) -> HttpResponse {
    println!("request: {:?}", req);
    println!("model: {:?}", item);

    HttpResponse::Ok().json(&item.0) // <- send json response
}

const MAX_SIZE: usize = 262_144; // max payload size is 256k

/// This handler manually load request payload and parse json object
async fn index_manual(mut payload: web::types::Payload) -> Result<HttpResponse, Error> {
    // payload is a stream of Bytes objects
    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow").into());
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<MyObj>(&body)?;
    Ok(HttpResponse::Ok().json(&obj)) // <- send response
}

/// This handler manually load request payload and parse json-rust
async fn index_mjsonrust(body: Bytes) -> Result<HttpResponse, Error> {
    // body is loaded, now we can deserialize json-rust
    let result = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
    let injson: JsonValue = match result {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e.to_string() },
    };
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(injson.dump()))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    web::server(async || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .state(web::types::JsonConfig::default().limit(4096)) // <- limit size of the payload (global configuration)
            .service((
                web::resource("/extractor").route(web::post().to(index)),
                web::resource("/extractor2")
                    .state(web::types::JsonConfig::default().limit(1024)) // <- limit size of the payload (resource level)
                    .route(web::post().to(extract_item)),
                web::resource("/manual").route(web::post().to(index_manual)),
                web::resource("/mjsonrust").route(web::post().to(index_mjsonrust)),
                web::resource("/").route(web::post().to(index)),
            ))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::web::{self, test, App};
    use ntex::{http, util::Bytes};

    #[ntex::test]
    async fn test_index() -> Result<(), Error> {
        let app = test::init_service(
            App::new().service(web::resource("/").route(web::post().to(index))),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&MyObj {
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
}
