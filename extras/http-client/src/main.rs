//! Download a Wikipedia image over HTTPS and return its buffered body.
//!
//! Look at `Client::builder` and `SslConnector` for OpenSSL setup, and
//! `web::AppState` for sharing the client. The handler buffers up to
//! 20,000,000 bytes rather than streaming the upstream response.
//! Run with `cargo run` and request <http://127.0.0.1:3000>.

use ntex::web::{self, App, HttpResponse};
use ntex::{client::Client, connect::openssl::SslConnector, SharedCfg};
use openssl::ssl;

#[derive(Clone)]
struct Download {
    client: Client,
    url: String,
}

type AppState = web::AppState<Download>;

async fn index(state: &AppState, _: ()) -> HttpResponse {
    let now = std::time::Instant::now();
    let payload = state
        .client
        .get(state.url.as_str())
        .send()
        .await
        .unwrap()
        .body()
        .limit(20_000_000) // sets max allowable payload size
        .await
        .unwrap();

    println!(
        "awc time elapsed while reading bytes into memory: {} ms",
        now.elapsed().as_millis()
    );

    HttpResponse::Ok().content_type("image/jpeg").body(payload)
}

fn app_config(config: &mut web::ServiceConfig<AppState>) {
    config.service(web::resource("/").to_with_state(index));
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let port = 3000;

    web::server(async |_| {
        let connector = SslConnector::new(
            ssl::SslConnector::builder(ssl::SslMethod::tls())
                .unwrap()
                .build(),
        );

        let client = Client::builder()
            .secure_connector(connector)
            .build(SharedCfg::default());

        App::new()
            .configure(app_config)
            .build_with(AppState::new(Download {
            client,
            url: "https://upload.wikimedia.org/wikipedia/commons/f/ff/Pizigani_1367_Chart_10MB.jpg"
                .to_owned(),
        }))
    })
    .bind(("0.0.0.0", port), SharedCfg::new("AWC"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{header, StatusCode};
    use ntex::web::test;

    /// STATEMENT: The index route downloads the upstream body and returns it as JPEG bytes.
    ///
    /// HOW:
    /// - Start a local upstream serving a small binary fixture instead of contacting Wikipedia.
    /// - Initialize the real app with that URL and request `/`.
    /// - Check HTTP 200, the JPEG content type, and the exact binary response body.
    #[ntex::test]
    async fn index_downloads_local_image() {
        let upstream = test::server(async |_| {
            App::new().route(
                "/image",
                web::get()
                    .to(async || HttpResponse::Ok().body(&b"\xff\xd8local image\xff\xd9"[..])),
            )
        });
        let app = test::init_service_st(
            AppState::new(Download {
                client: Client::new(),
                url: upstream.url("/image"),
            }),
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            test::read_body(resp).await.as_ref(),
            b"\xff\xd8local image\xff\xd9"
        );
    }

    /// STATEMENT: Unknown routes return HTTP 404 without attempting a download.
    ///
    /// HOW:
    /// - Initialize the real app with an unusable download URL and request `/missing`.
    /// - Check HTTP 404, showing that the download handler was not called.
    #[ntex::test]
    async fn unknown_route_is_not_found() {
        let app = test::init_service_st(
            AppState::new(Download {
                client: Client::new(),
                url: String::new(),
            }),
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::get().uri("/missing").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
