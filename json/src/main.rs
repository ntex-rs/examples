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

/// This handler uses json extractor
async fn index(item: types::Json<MyObj>) -> HttpResponse {
    println!("model: {:?}", &item);
    HttpResponse::Ok().json(&item.0) // <- send response
}

/// This handler uses json extractor with limit
async fn extract_item(item: types::Json<MyObj>, req: HttpRequest) -> HttpResponse {
    println!("request: {:?}", req);
    println!("model: {:?}", item);

    HttpResponse::Ok().json(&item.0) // <- send json response
}

const MAX_SIZE: usize = 262_144; // max payload size is 256k

/// This handler manually load request payload and parse json object
async fn index_manual(mut payload: types::Payload) -> Result<HttpResponse, WebError> {
    // payload is a stream of Bytes objects
    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(WebError::from_err)?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(WebError::from_err(error::ErrorBadRequest("overflow")));
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<MyObj>(&body).map_err(WebError::from_err)?;
    Ok(HttpResponse::Ok().json(&obj)) // <- send response
}

/// This handler manually load request payload and parse json-rust
async fn index_mjsonrust(body: Bytes) -> Result<HttpResponse, WebError> {
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
    env_logger::init();

    let cfg = ntex::SharedCfg::new("JSON")
        .add(
            web::WebAppConfig::new().set_state(types::JsonConfig::default().limit(4096)), // <- limit size of the payload (global configuration)
        )
        .build();
    let cfg2 = cfg.clone();

    web::server(async move |_| {
        App::new()
            .config(cfg2.get())
            // enable logger
            .middleware(middleware::Logger::default())
            .service((
                web::resource("/extractor").route(web::post().to(index)),
                web::resource("/extractor2").route(web::post().to(extract_item)),
                web::resource("/manual").route(web::post().to(index_manual)),
                web::resource("/mjsonrust").route(web::post().to(index_mjsonrust)),
                web::resource("/").route(web::post().to(index)),
            ))
    })
    .bind("127.0.0.1:8080", cfg)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::web::{self, App, test};
    use ntex::{http, util::Bytes};

    #[ntex::test]
    async fn test_index() -> Result<(), Error> {
        let app =
            test::init_service(App::new().service(web::resource("/").route(web::post().to(index))))
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
