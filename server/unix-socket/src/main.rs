//! Serve HTTP over a Unix domain socket instead of TCP.
//!
//! `web::HttpServer::bind_uds` binds `/tmp/ntex-uds.socket` on Unix and uses
//! the default server worker pool. Other platforms print `not supported`
//! and exit without starting a server.

#[cfg(unix)]
use ntex::web::{self, App, HttpRequest, middleware};

#[cfg(unix)]
async fn index(_req: HttpRequest) -> &'static str {
    "Hello world!"
}

#[cfg(unix)]
fn config(cfg: &mut web::ServiceConfig) {
    cfg.service((
        web::resource("/index.html").route(web::get().to(|| async { "Hello world!" })),
        web::resource("/").to(index),
    ));
}

#[ntex::main]
#[cfg(unix)]
async fn main() -> std::io::Result<()> {
    unsafe {
        ::std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(config)
    })
    .bind_uds("/tmp/ntex-uds.socket", ntex::SharedCfg::new("UDS"))?
    .run()
    .await
}

#[cfg(not(unix))]
fn main() -> std::io::Result<()> {
    println!("not supported");
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use ntex::http::StatusCode;
    use ntex::time::{Millis, timeout};
    use ntex::web::test::{self, TestRequest};

    use super::*;

    struct SocketPath(PathBuf);

    impl Drop for SocketPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// STATEMENT: Both Unix-server routes return their greeting in process.
    ///
    /// HOW:
    /// - Initialize the real routes without a listener.
    /// - Request `/` and `/index.html` and check `200 OK` and both greeting bodies.
    #[ntex::test]
    async fn greeting_routes_return_hello() {
        let app = test::init_service(App::new().configure(config)).await;
        for path in ["/", "/index.html"] {
            let response =
                test::call_service(&app, TestRequest::get().uri(path).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(test::read_body(response).await, "Hello world!");
        }
    }

    /// STATEMENT: Unknown Unix-server routes return an empty `404 Not Found` response.
    ///
    /// HOW:
    /// - Initialize the real app and request `/missing`.
    /// - Check the response status and empty body.
    #[ntex::test]
    async fn unknown_route_is_404() {
        let app = test::init_service(App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }

    /// STATEMENT: The real app serves HTTP through a Unix domain socket.
    ///
    /// HOW:
    /// - Bind a unique socket in the crate directory with a cleanup guard and start a test server.
    /// - Send an HTTP request with bounded socket I/O and check the status and greeting.
    /// - Stop the server, clean up the socket and confirm its path is gone.
    #[ntex::test]
    async fn unix_socket_serves_http_and_is_cleaned_up() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("test-{}-{unique}.sock", std::process::id()));
        let socket = SocketPath(path.clone());
        let listener = UnixListener::bind(&path).unwrap();
        let server =
            ntex::server::build_test_server(ntex::server::NoConfig, async move |builder| {
                builder
                    .listen_uds(
                        "test-uds",
                        listener,
                        ntex::SharedCfg::new("UDS-TEST"),
                        async |_| {
                            ntex::http::HttpService::new(App::new().configure(config).build())
                        },
                    )
                    .unwrap()
            });

        let mut connection = UnixStream::connect(&path).unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        connection
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        connection
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        connection.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
        assert!(response.ends_with("\r\n\r\nHello world!"), "{response}");
        drop(connection);
        timeout(Millis(1000), server.server().stop(true))
            .await
            .unwrap();
        drop(server);
        drop(socket);
        assert!(!path.exists());
    }
}
