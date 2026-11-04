// Deeply nested service types (TLS + HttpService + App + middlewares)
// exceed the default query depth limit.
#![recursion_limit = "256"]

use std::convert::Infallible;

use ntex::web::{self, middleware, App, HttpRequest, HttpResponse};
use openssl::ssl::{SslAcceptor, SslMethod};
use tls_x509_example::{load_identity, Error};

/// Returns request information as a response.
async fn index(req: HttpRequest) -> Result<HttpResponse, Infallible> {
    println!("{:?}", req);
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body("Welcome!"))
}

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    println!("Started http server: 127.0.0.1:8443");

    let identity = load_identity("../certs/server")?;
    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
    ntex::tls::openssl::set_identity(&mut builder, &identity)?;

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
    })
    .bind_openssl("127.0.0.1:8443", builder, ntex::SharedCfg::new("SSL"))?
    .run()
    .await?;
    Ok(())
}
