//! Render a name form and personalized greeting with Askama.
//!
//! `#[derive(Template)]` compiles the HTML templates, while
//! `web::types::Query` selects the form or greeting from the `name` parameter.
//! The handler renders the template into an HTML `HttpResponse`.

use std::collections::HashMap;

use askama::Template;
use ntex::web::{self, App, HttpResponse, WebError};

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    name: &'a str,
    text: &'a str,
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index;

#[web::get("/")]
async fn index(
    query: web::types::Query<HashMap<String, String>>,
) -> Result<HttpResponse, WebError> {
    let s = if let Some(name) = query.get("name") {
        UserTemplate {
            name,
            text: "Welcome!",
        }
        .render()
        .unwrap()
    } else {
        Index.render().unwrap()
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

/// Registers the example's route.
fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(index);
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    // Start the HTTP server.
    web::server(async move |_| App::new().configure(config))
        .bind("127.0.0.1:8080", ntex::SharedCfg::new("S"))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};

    /// STATEMENT: The index renders an HTML form when no name is supplied.
    ///
    /// HOW:
    /// - Initialize the real routes and request `/` without query parameters.
    /// - Check `200`, HTML content type, the welcome heading, and the name field.
    #[ntex::test]
    async fn index_renders_name_form() {
        let app = test::init_service(App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html"
        );
        let body = test::read_body(response).await;
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("<h1>Welcome!</h1>"));
        assert!(body.contains("name=\"name\""));
    }

    /// STATEMENT: The greeting renders the submitted name with HTML escaping.
    ///
    /// HOW:
    /// - Request `/?name=%3Cb%3EAlice%3C%2Fb%3E` through the real app.
    /// - Check `200`, HTML content type, the escaped greeting, and the welcome text.
    #[ntex::test]
    async fn greeting_escapes_submitted_name() {
        let app = test::init_service(App::new().configure(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/?name=%3Cb%3EAlice%3C%2Fb%3E")
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html"
        );
        let body = test::read_body(response).await;
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("Hi, &#60;b&#62;Alice&#60;"), "{body}");
        assert!(body.contains("Welcome!"));
        assert!(!body.contains("<b>Alice</b>"));
    }
    /// STATEMENT: Unknown paths are not rendered as the index template.
    ///
    /// HOW:
    /// - Send `GET /missing` through the real routes and check `404` and an empty body.
    #[ntex::test]
    async fn unknown_route_is_not_found() {
        let app = test::init_service(App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }
}
