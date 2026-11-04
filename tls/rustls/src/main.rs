// Deeply nested service types (TLS + HttpService + App + middlewares)
// exceed the default query depth limit.
#![recursion_limit = "256"]

use std::sync::Arc;

use ntex::web::{self, App, HttpRequest, HttpResponse, middleware};
use ntex_files::Files;
use rustls::{ServerConfig, sign::SingleCertAndKey};
use tls_x509_example::{Error, load_identity};

/// Returns request information as a response.
async fn index(req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Welcome!")
}

#[ntex::main]
async fn main() -> Result<(), Error> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    env_logger::init();

    let identity = load_identity("../certs/server")?;
    let key = ntex::tls::rustls::certified_key(&identity)?;
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(SingleCertAndKey::from(key)));

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
    .await?;
    Ok(())
}
