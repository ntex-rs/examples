use futures::executor;
use ntex::web::{self, middleware, App, HttpResponse};
use std::{sync::mpsc, thread};

#[web::get("/hello")]
async fn hello() -> &'static str {
    "Hello world!"
}

#[web::post("/stop")]
async fn stop(stopper: web::types::State<mpsc::Sender<()>>) -> HttpResponse {
    // make request that sends message through the Sender
    stopper.send(()).unwrap();

    HttpResponse::NoContent().finish()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // create a channel
    let (tx, rx) = mpsc::channel::<()>();
    let _stopper = tx.clone();

    let bind = "127.0.0.1:8080";

    // start server as normal but don't .await after .run() yet
    let server = web::server(async move || {
        // give the server a Sender in .data
        let stopper = tx.clone();

        App::new()
            .state(stopper)
            .wrap(middleware::Logger::default())
            .service((hello, stop))
    })
    .bind(bind)?
    .run();

    // clone the Server handle
    let srv = server.clone();
    thread::spawn(move || {
        // wait for shutdown signal
        rx.recv().unwrap();

        // stop server gracefully
        executor::block_on(srv.stop(true))
    });

    // run server
    server.await
}
