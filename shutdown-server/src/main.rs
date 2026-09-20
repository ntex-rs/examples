use futures::executor;
use ntex::web::{self, App, HttpResponse, middleware};
use std::{sync::mpsc, thread};

type AppState = web::AppState<mpsc::Sender<()>>;

#[web::get("/hello", state = AppState)]
async fn hello() -> &'static str {
    "Hello world!"
}

async fn stop(stopper: &AppState, _: ()) -> HttpResponse {
    // Notify the control thread that the server should stop.
    stopper.send(()).unwrap();

    HttpResponse::NoContent().build()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    // Create the shutdown channel.
    let (tx, rx) = mpsc::channel::<()>();
    let _stopper = tx.clone();

    let bind = "127.0.0.1:8080";

    // Start the server without awaiting it yet.
    let server = web::server(async move |_| {
        // Store the shutdown sender in application state.
        let stopper = tx.clone();

        App::new()
            .middleware(middleware::Logger::default())
            .service((
                hello,
                web::resource("/stop").route(web::post().to_with_state(stop)),
            ))
            .build_with(AppState::new(stopper))
    })
    .bind(bind, ntex::SharedCfg::new("SHUTDOWN"))?
    .run();

    // Clone the server handle for the control thread.
    let srv = server.clone();
    thread::spawn(move || {
        // Wait for the handler to request shutdown.
        rx.recv().unwrap();

        // Stop the server gracefully.
        executor::block_on(srv.stop(true))
    });

    // Wait until the server stops.
    server.await
}
