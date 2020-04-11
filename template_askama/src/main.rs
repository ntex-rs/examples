use std::collections::HashMap;

use askama::Template;
use ntex::web::{self, App, Error, HttpResponse};

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
) -> Result<HttpResponse, Error> {
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

#[ntex::main]
async fn main() -> std::io::Result<()> {
    // start http server
    web::server(move || App::new().service(index))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
