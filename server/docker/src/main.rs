//! A minimal ntex HTTP server with a multi-stage Dockerfile.
//!
//! `web::server` registers two `#[web::get]` handlers at `/` and `/again`.
//! The server binds to `0.0.0.0:5000`; see the README for container build caveats.

use ntex::web::{self, App, HttpResponse};

#[web::get("/")]
async fn index() -> HttpResponse {
    println!("GET: /");
    HttpResponse::Ok().body("Hello world!")
}

#[web::get("/again")]
async fn again() -> HttpResponse {
    println!("GET: /again");
    HttpResponse::Ok().body("Hello world again!")
}

fn config(cfg: &mut web::ServiceConfig) {
    cfg.service((index, again));
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    println!("Starting ntex-web server");

    web::server(async |_| App::new().configure(config))
        .bind("0.0.0.0:5000", ntex::SharedCfg::new("S"))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use ntex::http::StatusCode;
    use ntex::web::test::{self, TestRequest};

    use super::*;

    /// STATEMENT: Both greeting routes return their respective messages.
    ///
    /// HOW:
    /// - Initialize the real routes without a listener.
    /// - Request `/` and `/again` and check `200 OK` and each greeting body.
    #[ntex::test]
    async fn greeting_routes_return_their_messages() {
        let app = test::init_service(App::new().configure(config)).await;
        for (path, body) in [("/", "Hello world!"), ("/again", "Hello world again!")] {
            let response =
                test::call_service(&app, TestRequest::get().uri(path).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(test::read_body(response).await, body);
        }
    }

    /// STATEMENT: Unknown routes return `404 Not Found`.
    ///
    /// HOW:
    /// - Initialize the real routes and request `/missing`.
    /// - Check the status and empty response body.
    #[ntex::test]
    async fn unknown_route_is_404() {
        let app = test::init_service(App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }
}
