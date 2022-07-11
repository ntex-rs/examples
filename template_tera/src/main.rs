use std::collections::HashMap;

use ntex::web::{self, error, middleware, App, Error, HttpResponse};
use tera::Tera;

// store tera template in application state
#[web::get("/")]
async fn index(
    tmpl: web::types::State<tera::Tera>,
    query: web::types::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let s = if let Some(name) = query.get("name") {
        // submitted form
        let mut ctx = tera::Context::new();
        ctx.insert("name", &name.to_owned());
        ctx.insert("text", &"Welcome!".to_owned());
        tmpl.render("user.html", &ctx)
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    } else {
        tmpl.render("index.html", &tera::Context::new())
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    web::server(|| {
        let tera =
            Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();

        App::new()
            .state(tera)
            .wrap(middleware::Logger::default()) // enable logger
            .service(index)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
