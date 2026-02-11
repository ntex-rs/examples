#![allow(dead_code, clippy::type_complexity)]

use ntex::web;

mod read_request_body;
mod read_response_body;
mod redirect;
mod simple;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    web::server(async || {
        web::App::new()
            .filter(|req: web::WebRequest<_>| async move {
                println!("Hi from start. You requested: {}", req.path());
                Ok(req)
            })
            .middleware(simple::SayHi)
            .middleware(read_request_body::Logging)
            .middleware(read_response_body::Logging)
            .middleware(redirect::CheckLogin)
            .service(web::resource("/login").to(|| async {
                "You are on /login. Go to src/redirect.rs to change this behavior."
            }))
            .service(web::resource("/").to(|| async {
                "Hello, middleware! Check the console where the server is run."
            }))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
