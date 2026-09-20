#[macro_use]
extern crate serde_json;

use handlebars::Handlebars;
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

#[ntex::main]
async fn main() -> io::Result<()> {
    // Compile the templates once, then share them with each server worker.
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./static/templates")
        .unwrap();
    let handlebars_ref = Arc::new(handlebars);

    web::server(async move |_| {
        App::new()
            .route("/", web::get().to_with_state(index))
            .route("/{user}/{data}", web::get().to_with_state(user))
            .build_with(AppState::new(handlebars_ref.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("S"))?
    .run()
    .await
}
