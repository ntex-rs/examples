//! Request graceful ntex server shutdown through an HTTP handler.
//!
//! `web::AppState` holds a channel sender used by `POST /stop`.
//! A control thread receives the message and awaits `Server::stop(true)`;
//! `GET /hello` is a regular greeting route.

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

fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.service((
        hello,
        web::resource("/stop").route(web::post().to_with_state(stop)),
    ));
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
            .configure(config)
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

#[cfg(test)]
mod tests {
    use ntex::http::StatusCode;
    use ntex::time::{Millis, timeout};
    use ntex::web::test::{self, TestRequest};

    use super::*;

    /// STATEMENT: The greeting route does not request shutdown.
    ///
    /// HOW:
    /// - Initialize the real routes with a shutdown channel and request `/hello`.
    /// - Check the successful greeting and confirm the channel has no message.
    #[ntex::test]
    async fn greeting_does_not_request_shutdown() {
        let (tx, rx) = mpsc::channel();
        let app = test::init_service_st(AppState::new(tx), App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/hello").to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(test::read_body(response).await, "Hello world!");
        assert_eq!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    /// STATEMENT: Posting to the stop route sends exactly one shutdown notification.
    ///
    /// HOW:
    /// - Initialize the app with a shutdown channel and send `POST /stop`.
    /// - Check an empty `204` response, one channel message and no second message.
    #[ntex::test]
    async fn stop_notifies_controller() {
        let (tx, rx) = mpsc::channel();
        let app = test::init_service_st(AppState::new(tx), App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::post().uri("/stop").to_request()).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(test::read_body(response).await.is_empty());
        assert_eq!(rx.try_recv(), Ok(()));
        assert_eq!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    /// STATEMENT: Reading the stop route does not trigger shutdown.
    ///
    /// HOW:
    /// - Send `GET /stop` to the real routes with a shutdown channel.
    /// - Check `405 Method Not Allowed`, an empty body and no shutdown message.
    #[ntex::test]
    async fn stop_rejects_get_without_notification() {
        let (tx, rx) = mpsc::channel();
        let app = test::init_service_st(AppState::new(tx), App::new().configure(config)).await;
        let response = test::call_service(&app, TestRequest::get().uri("/stop").to_request()).await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(test::read_body(response).await.is_empty());
        assert_eq!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    /// STATEMENT: An HTTP shutdown request allows the controller to stop a live server.
    ///
    /// HOW:
    /// - Start the real stateful app on an ephemeral test server and send `POST /stop`.
    /// - Check `204`, consume the notification and await a bounded graceful stop.
    #[ntex::test]
    async fn shutdown_request_stops_server() {
        let (tx, rx) = mpsc::channel();
        let server = test::server(async move |_| {
            App::new()
                .configure(config)
                .build_with(AppState::new(tx.clone()))
        });
        let response = server.post("/stop").send().await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(server.load_body(response).await.unwrap().is_empty());
        assert_eq!(rx.try_recv(), Ok(()));
        timeout(Millis(1000), server.stop()).await.unwrap();
    }
}
