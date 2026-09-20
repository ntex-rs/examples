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

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async |_| {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();

        App::new()
            .middleware(middleware::Logger::default()) // enable logger
            .route("/", web::get().to_with_state(index))
            .build_with(AppState::new(tera))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("S"))?
    .run()
    .await
}
