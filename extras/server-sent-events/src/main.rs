use std::sync::Arc;
use std::{pin::Pin, sync::Mutex, task::Context, task::Poll, time::Duration};

use futures::Stream;
use ntex::web::{self, App, HttpResponse, WebError};
use ntex::{time::interval, util::Bytes};
use tokio::sync::mpsc::{Receiver, Sender, channel};

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let data = Broadcaster::create();

    web::server(async move |_| {
        App::new()
            .route("/", web::get().to(index))
            .route("/events", web::get().to_with_state(new_client))
            .route("/broadcast/{msg}", web::get().to_with_state(broadcast))
            .build_with(AppState::new(data.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("SSE"))?
    .run()
    .await
}

async fn index() -> HttpResponse {
    let content = include_str!("index.html");

    HttpResponse::Ok()
        .header("content-type", "text/html")
        .body(content)
}

type AppState = web::AppState<Arc<Mutex<Broadcaster>>>;

async fn new_client(broadcaster: &AppState, _: ()) -> HttpResponse {
    let rx = broadcaster.lock().unwrap().new_client();

    HttpResponse::Ok()
        .header("content-type", "text/event-stream")
        .no_chunking()
        .streaming(rx)
}

async fn broadcast(broadcaster: &AppState, _: (), msg: web::types::Path<String>) -> HttpResponse {
    broadcaster.lock().unwrap().send(&msg.into_inner());

    HttpResponse::Ok().body("msg sent")
}

struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    fn create() -> Arc<Mutex<Self>> {
        // Data ≃ Arc
        let me = Arc::new(Mutex::new(Broadcaster::new()));

        // ping clients every 10 seconds to see if they are alive
        Broadcaster::spawn_ping(me.clone());

        me
    }

    fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    fn spawn_ping(me: Arc<Mutex<Self>>) {
        ntex::rt::spawn(async move {
            loop {
                let task = interval(Duration::from_secs(10));
                task.tick().await;
                me.lock().unwrap().remove_stale_clients();
            }
        });
    }

    fn remove_stale_clients(&mut self) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Bytes::from("data: ping\n\n"));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        tx.try_send(Bytes::from("data: connected\n\n")).unwrap();

        self.clients.push(tx);
        Client(rx)
    }

    fn send(&self, msg: &str) {
        let msg = Bytes::from(["data: ", msg, "\n\n"].concat());

        for client in self.clients.iter() {
            client.clone().try_send(msg.clone()).unwrap_or(());
        }
    }
}

// wrap Receiver in own type, with correct error type
struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, WebError<AppState>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_recv(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
