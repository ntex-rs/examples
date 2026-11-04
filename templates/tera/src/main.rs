//! Render a name form and greeting with Tera and ntex application state.
//!
//! Each worker loads templates with `Tera::load_from_glob` and stores the
//! engine in `web::AppState`. The handler extracts `name` from the query
//! and converts template rendering errors into HTTP 500 responses.

use std::collections::HashMap;

use ntex::web::{self, App, HttpResponse, error, middleware, types};
use tera::Tera;

type Error = web::WebError<AppState>;
type AppState = web::AppState<tera::Tera>;

async fn index(
    tmpl: &AppState,
    _: (),
    query: types::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let s = if let Some(name) = query.get("name") {
        // submitted form
        let mut ctx = tera::Context::new();
        ctx.insert("name", &name.to_owned());
        ctx.insert("text", &"Welcome!".to_owned());
        tmpl.render("user.html", &ctx)
            .map_err(|_| Error::from_err(error::ErrorInternalServerError("Template error")))?
    } else {
        tmpl.render("index.html", &tera::Context::new())
            .map_err(|_| Error::from_err(error::ErrorInternalServerError("Template error")))?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

/// Loads the templates from this crate's directory.
fn template_state() -> AppState {
    let mut tera = Tera::new();
    tera.load_from_glob(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
        .unwrap();
    AppState::new(tera)
}

/// Registers the template-rendering route.
fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.route("/", web::get().to_with_state(index));
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async |_| {
        App::new()
            .middleware(middleware::Logger::default()) // enable logger
            .configure(config)
            .build_with(template_state())
    })
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
        let app = test::init_service_st(template_state(), App::new().configure(config)).await;
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
        let app = test::init_service_st(template_state(), App::new().configure(config)).await;
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
        assert!(body.contains("Hi, &lt;b&gt;Alice&lt;"));
        assert!(body.contains("Welcome!"));
        assert!(!body.contains("<b>Alice</b>"));
    }
    /// STATEMENT: Template-rendering failures become a generic internal-server-error response.
    ///
    /// HOW:
    /// - Initialize the real route with an empty `Tera` registry and request `/`.
    /// - Check `500` and the public `Template error` body rather than internal template details.
    #[ntex::test]
    async fn missing_template_returns_internal_server_error() {
        let app =
            test::init_service_st(AppState::new(Tera::new()), App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(test::read_body(response).await, "Template error");
    }
}
