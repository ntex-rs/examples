//! HTTPS web server using rustls with an explicit ring provider and `bind_rustls`.
//!
//! Run from `tls/rustls`: `cargo run --bin rustls-server`

// Deeply nested service types (TLS + HttpService + App + middlewares)
// exceed the default query depth limit.
#![recursion_limit = "256"]

use ntex::web::{self, App, HttpRequest, HttpResponse, middleware};
use rustls_example::server_config;
use tls_x509_example::{Error, load_identity};

/// Returns request information as a response.
async fn index(req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Welcome!")
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let identity = load_identity("../certs/server")?;
    let config = server_config(&identity)?;

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(configure)
    })
    .bind_rustls("127.0.0.1:8443", &config, ntex::SharedCfg::new("RUSTLS"))?
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
        SharedCfg,
        client::Client,
        http::StatusCode,
        time::{Seconds, timeout},
        web::test,
    };
    use rustls_example::client_config;

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
            test::config().rustls(server_config(&identity).unwrap()),
            async |_| App::new().configure(configure),
        );
        let client = Client::builder()
            .rustls(client_config(ca.to_pem().as_bytes(), None).unwrap())
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
