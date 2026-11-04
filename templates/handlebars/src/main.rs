//! Render Handlebars templates with shared ntex application state.
//!
//! An `Arc<Handlebars>` registry in `web::AppState` is shared across workers.
//! `GET /` and `GET /{user}/{data}` render templates with JSON values.
//! Templates are loaded from the crate's `static/templates` directory.

#[macro_use]
extern crate serde_json;

use handlebars::{DirectorySourceOptionsBuilder, Handlebars};
use ntex::web::{self, App, HttpResponse, types};
use std::{io, sync::Arc};

type AppState = web::AppState<Arc<Handlebars<'static>>>;

async fn index(hb: &AppState, _: ()) -> HttpResponse {
    let data = json!({
        "name": "Handlebars"
    });
    let body = hb.render("index", &data).unwrap();

    HttpResponse::Ok().body(body)
}

async fn user(hb: &AppState, _: (), info: types::Path<(String, String)>) -> HttpResponse {
    let data = json!({
        "user": info.0,
        "data": info.1
    });
    let body = hb.render("user", &data).unwrap();

    HttpResponse::Ok().body(body)
}

/// Loads the template registry independently of the working directory.
fn template_state() -> AppState {
    // Compile the templates once, then share them with each server worker.
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(
            concat!(env!("CARGO_MANIFEST_DIR"), "/static/templates"),
            DirectorySourceOptionsBuilder::default()
                .tpl_extension(".html")
                .build()
                .unwrap(),
        )
        .unwrap();
    AppState::new(Arc::new(handlebars))
}

/// Registers routes that render the shared templates.
fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.route("/", web::get().to_with_state(index))
        .route("/{user}/{data}", web::get().to_with_state(user));
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let state = template_state();

    web::server(async move |_| App::new().configure(config).build_with(state.clone()))
        .bind("127.0.0.1:8080", ntex::SharedCfg::new("S"))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::StatusCode;
    use ntex::web::test::{self, TestRequest};

    /// STATEMENT: The index renders the shared Handlebars template.
    ///
    /// HOW:
    /// - Load the real registry, initialize the configured routes with state, and request `/`.
    /// - Check `200`, the rendered title, and the heading.
    #[ntex::test]
    async fn index_renders_handlebars_page() {
        let app = test::init_service_st(template_state(), App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("<title>Handlebars Example</title>"));
        assert!(body.contains("<h1>Handlebars example</h1>"));
    }

    /// STATEMENT: The user template renders path values and escapes HTML in them.
    ///
    /// HOW:
    /// - Send an encoded HTML user name and a data value to the real `/{user}/{data}` route.
    /// - Check `200`, the escaped name, the data text, and absence of raw HTML.
    #[ntex::test]
    async fn user_page_renders_and_escapes_path_values() {
        let app = test::init_service_st(template_state(), App::new().configure(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri("/%3Cb%3EEmma%3C%2Fb%3E/documents")
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("Welcome back, &lt;b&gt;Emma&lt;"));
        assert!(body.contains("Here's your documents."));
        assert!(!body.contains("<b>Emma</b>"));
    }

    /// STATEMENT: An incomplete user path returns a routing error instead of a rendered page.
    ///
    /// HOW:
    /// - Request `/Emma` with the real routes and registry and check `404` and an empty body.
    #[ntex::test]
    async fn incomplete_user_path_is_not_found() {
        let app = test::init_service_st(template_state(), App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/Emma").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }
}
