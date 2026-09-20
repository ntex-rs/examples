use std::fs::File;
use std::io::BufReader;

use ntex::web::{self, App, HttpRequest, HttpResponse, middleware};
use ntex_files::Files;
use rustls::ServerConfig;
use rustls_pemfile::certs;

/// Returns request information as a response.
async fn index(req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Welcome!")
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
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

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            // Handle every method at this path.
            .service(web::resource("/index.html").to(index))
            // with path parameters
            .service(web::resource("/").route(web::get().to(|| async {
                HttpResponse::Found()
                    .header("LOCATION", "/index.html")
                    .build()
            })))
            .service(Files::new("/static", "static"))
    })
    .bind_rustls("127.0.0.1:8443", &config, ntex::SharedCfg::new("RUSTLS"))?
    .run()
    .await
}
