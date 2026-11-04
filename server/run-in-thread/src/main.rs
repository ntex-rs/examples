//! Run an ntex server in a separate thread and stop it from `main`.
//!
//! A `std::sync::mpsc` channel sends a cloned `ntex::server::Server` handle
//! to the main thread. After ten seconds, `Server::stop(true)` requests
//! graceful shutdown and waits for it to finish.

use std::sync::mpsc;
use std::{thread, time};

use ntex::server::Server;
use ntex::web::{self, App, HttpRequest, middleware};

async fn index(req: HttpRequest) -> &'static str {
    println!("REQ: {:?}", req);
    "Hello world!"
}

fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/index.html").to(|| async { "Hello world!" }))
        .service(web::resource("/").to(index));
}

#[ntex::main]
async fn run_app(tx: mpsc::Sender<Server>) -> std::io::Result<()> {
    // srv is server controller type, `dev::Server`
    let srv = web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(config)
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("THREAD"))?
    .run();

    // send server controller to main thread
    let _ = tx.send(srv.clone());

    // run future
    srv.await
}

#[ntex::main]
async fn main() {
    unsafe {
        std::env::set_var("RUST_LOG", "trace");
    }
    env_logger::init();

    let (tx, rx) = mpsc::channel();

    println!("START SERVER");
    thread::spawn(move || {
        let _ = run_app(tx);
    });

    let srv = rx.recv().unwrap();

    println!("WAITING 10 SECONDS");
    thread::sleep(time::Duration::from_secs(10));

    println!("STOPPING SERVER");
    // init stop server and wait until server gracefully exit
    srv.stop(true).await;
}

#[cfg(test)]
mod tests {
    use ntex::http::StatusCode;
    use ntex::time::{Millis, timeout};
    use ntex::web::test::{self, TestRequest};

    use super::*;

    /// STATEMENT: Both routes return the greeting without needing a server thread.
    ///
    /// HOW:
    /// - Initialize the real routes in process.
    /// - Request `/` and `/index.html` and check successful greeting responses.
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

    /// STATEMENT: Unknown routes return an empty `404 Not Found` response.
    ///
    /// HOW:
    /// - Initialize the real app and request `/missing`.
    /// - Check the status and response body.
    #[ntex::test]
    async fn unknown_route_is_404() {
        let app = test::init_service(App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }

    /// STATEMENT: A server on a separate test thread can be stopped gracefully.
    ///
    /// HOW:
    /// - Start the real app on an ephemeral test server and verify its greeting.
    /// - Await graceful shutdown with a one-second timeout, without the example's sleep.
    #[ntex::test]
    async fn threaded_server_stops_gracefully() {
        let server = test::server(async |_| App::new().configure(config));
        let response = server.get("/").send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(server.load_body(response).await.unwrap(), "Hello world!");
        timeout(Millis(1000), server.stop()).await.unwrap();
    }
}
