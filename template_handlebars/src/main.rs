#[macro_use]
extern crate serde_json;

use handlebars::Handlebars;
use ntex::web::{self, App, HttpResponse};
use std::io;

// Macro documentation can be found in the ntex_macros crate
#[web::get("/")]
async fn index(hb: web::types::State<Handlebars<'_>>) -> HttpResponse {
    let data = json!({
        "name": "Handlebars"
    });
    let body = hb.render("index", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[web::get("/{user}/{data}")]
async fn user(
    hb: web::types::State<Handlebars<'_>>,
    info: web::types::Path<(String, String)>,
) -> HttpResponse {
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
    let handlebars_ref = web::types::State::new(handlebars);

    web::server(move || {
        App::new()
            .app_state(handlebars_ref.clone())
            .service((index, user))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
