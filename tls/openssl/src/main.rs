//! HTTPS web server using OpenSSL and `bind_openssl`.
//!
//! Run from `tls/openssl`: `cargo run --bin openssl-example`

// Deeply nested service types (TLS + HttpService + App + middlewares)
// exceed the default query depth limit.
#![recursion_limit = "256"]

use std::convert::Infallible;

use ntex::web::{self, middleware, App, HttpRequest, HttpResponse};
use openssl_example::server_config;
use tls_x509_example::{load_identity, Error};

/// Returns request information as a response.
async fn index(req: HttpRequest) -> Result<HttpResponse, Infallible> {
    println!("{:?}", req);
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body("Welcome!"))
}

/// Registers the welcome page and redirect.
fn configure(cfg: &mut web::ServiceConfig) {
    cfg // Handle every method at this path.
        .service(web::resource("/index.html").to(index))
        // with path parameters
        .service(web::resource("/").route(web::get().to(|| async {
            HttpResponse::Found()
                .header("LOCATION", "/index.html")
                .build()
        })));
}

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    println!("Started http server: 127.0.0.1:8443");

    let identity = load_identity("../certs/server")?;
    let builder = server_config(&identity)?;

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(configure)
    })
    .bind_openssl("127.0.0.1:8443", builder, ntex::SharedCfg::new("SSL"))?
    .run()
    .await?;
    Ok(())
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::{
        client::Client,
        http::StatusCode,
        time::{timeout, Seconds},
        web::test,
        SharedCfg,
    };
    use openssl_example::client_config;

    /// STATEMENT: The HTTPS application serves its welcome page, redirect, and missing-route response.
    ///
    /// HOW:
    /// - Generate identities in memory and start the real routes with the server TLS configuration.
    /// - Use the example's CA-verifying client configuration to request the page, root, and a missing path.
    /// - Check the welcome text and content type, redirect location, and `404` status within a timeout.
    #[ntex::test]
    async fn https_serves_real_routes() {
        let (ca, identity, _) = test_support::identities();
        let srv = test::server_with(
            test::config().openssl(server_config(&identity).unwrap().build()),
            async |_| App::new().configure(configure),
        );
        let client = Client::builder()
            .openssl(client_config(ca.to_pem().as_bytes(), None).unwrap())
            .build(SharedCfg::new("HTTPS-TEST"));
        timeout(Seconds(3), async {
            let response = client
                .get(srv.url("/index.html").as_str())
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers().get("content-type").unwrap(),
                "text/plain"
            );
            assert_eq!(response.body().await.unwrap(), "Welcome!");
            let response = client.get(srv.url("/").as_str()).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::FOUND);
            assert_eq!(response.headers().get("location").unwrap(), "/index.html");
            let response = client
                .get(srv.url("/missing").as_str())
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        })
        .await
        .unwrap();
    }
}
