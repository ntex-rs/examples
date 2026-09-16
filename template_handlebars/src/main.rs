#[macro_use]
extern crate serde_json;

use handlebars::Handlebars;
use ntex::web::{self, App, HttpResponse, types};
use std::{io, sync::Arc};

type AppState = web::AppState<Arc<Handlebars<'static>>>;

// Macro documentation can be found in the ntex_macros crate
#[web::get("/", state=AppState)]
async fn index(hb: types::State<AppState>) -> HttpResponse {
    let data = json!({
        "name": "Handlebars"
    });
    let body = hb.render("index", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[web::get("/{user}/{data}", state=AppState)]
async fn user(hb: types::State<AppState>, info: types::Path<(String, String)>) -> HttpResponse {
    let data = json!({
        "user": info.0,
        "data": info.1
    });
    let body = hb.render("user", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[ntex::main]
async fn main() -> io::Result<()> {
    // Handlebars uses a repository for the compiled templates. This object must be
    // shared between the application threads, and is therefore passed to the
    // Application Builder as an atomic reference-counted pointer.
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./static/templates")
        .unwrap();
    let handlebars_ref = Arc::new(handlebars);

    web::server(async move |_| {
        App::new()
            .service((index, user))
            .build_with(AppState::new(handlebars_ref.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("S"))?
    .run()
    .await
}
