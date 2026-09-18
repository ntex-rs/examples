use futures::executor;
use ntex::web::{self, App, HttpResponse, middleware};
use std::{sync::mpsc, thread};

type AppState = web::AppState<mpsc::Sender<()>>;

#[web::get("/hello", state = AppState)]
async fn hello() -> &'static str {
    "Hello world!"
}

#[web::post("/stop", state = AppState)]
async fn stop(stopper: web::types::State<AppState>) -> HttpResponse {
    // make request that sends message through the Sender
    stopper.send(()).unwrap();

    HttpResponse::NoContent().build()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    // create a channel
    let (tx, rx) = mpsc::channel::<()>();
    let _stopper = tx.clone();

    let bind = "127.0.0.1:8080";

    // start server as normal but don't .await after .run() yet
    let server = web::server(async move |_| {
        // give the server a Sender in .data
        let stopper = tx.clone();

        App::new()
            .middleware(middleware::Logger::default())
            .service((hello, stop))
            .build_with(AppState::new(stopper))
    })
    .bind(bind, ntex::SharedCfg::new("SHUTDOWN"))?
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
