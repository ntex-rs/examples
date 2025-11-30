use std::fs::File;
use std::io::BufReader;

use ntex::web::{self, middleware, App, HttpRequest, HttpResponse};
use ntex_files::Files;
use rustls::ServerConfig;
use rustls_pemfile::certs;

/// simple handle
async fn index(req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Welcome!")
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // load ssl keys
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    let key = rustls_pemfile::private_key(key_file).unwrap().unwrap();
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let cert_chain = certs(cert_file).map(|r| r.unwrap()).collect();
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .unwrap();

    web::server(async || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            // register simple handler, handle all methods
            .service(web::resource("/index.html").to(index))
            // with path parameters
            .service(web::resource("/").route(web::get().to(|| async {
                HttpResponse::Found()
                    .header("LOCATION", "/index.html")
                    .finish()
            })))
            .service(Files::new("/static", "static"))
    })
    .bind_rustls("127.0.0.1:8443", config)?
    .run()
    .await
}
