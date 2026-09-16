//! By default, `ntex` runs one `App` instance per logical CPU core.
//! When the server runs on `N` cores, `counter1` (global state protected by a
//! mutex) and `counter2` (global atomic state) are incremented every time the
//! endpoint is called. In contrast, each instance of `counter3` is incremented
//! only once every `N` requests on average because it is worker-local state and
//! the workload is distributed across the workers.
use std::sync::{Arc, Mutex, atomic::AtomicUsize, atomic::Ordering};
use std::{cell::Cell, io};

use ntex::web::{self, App, HttpRequest, HttpResponse, middleware, types};
use ntex::{SharedCfg, server::ServerAppConfig};

#[derive(Clone)]
struct AppState {
    counter1: Arc<Mutex<usize>>,
    counter2: Arc<AtomicUsize>,
    counter3: Cell<u32>,
}

impl web::State for AppState {
    type Error = web::DefaultError;
}

struct AppBuilder {
    counter1: Arc<Mutex<usize>>,
    counter2: Arc<AtomicUsize>,
}

impl AppBuilder {
    fn new() -> Self {
        Self {
            counter1: Arc::new(Mutex::new(0usize)),
            counter2: Arc::new(AtomicUsize::new(0usize)),
        }
    }
}

impl ServerAppConfig for AppBuilder {
    type State = AppState;

    // This method creates woroker state
    async fn create(&self) -> io::Result<Self::State> {
        Ok(AppState {
            counter1: self.counter1.clone(),
            counter2: self.counter2.clone(),

            // Create some thread-local state
            counter3: Cell::new(0),
        })
    }
}

/// simple handle
async fn index(st: types::State<AppState>, req: HttpRequest) -> HttpResponse {
    println!("{:?}", req);

    // Increment the counters
    *st.counter1.lock().unwrap() += 1;
    st.counter2.fetch_add(1, Ordering::SeqCst);
    st.counter3.set(st.counter3.get() + 1);

    let body = format!(
        "global mutex counter: {}, global atomic counter: {}, local counter: {}",
        *st.counter1.lock().unwrap(),
        st.counter2.load(Ordering::SeqCst),
        st.counter3.get(),
    );

    HttpResponse::Ok().body(body)
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    // Create the process-wide configuration before building the server.
    let cfg = AppBuilder::new();

    // Start the server with the application configuration. The factory
    // closure receives a reference to the worker-local state, which also
    // becomes the state of the App instance.
    web::server_with_config(cfg, async move |_: &AppState| {
        App::new()
            // Enable request logging.
            .middleware(middleware::Logger::default())
            // Register the request handler.
            .service(web::resource("/").to(index))
    })
    .bind("127.0.0.1:8080", SharedCfg::new("STATE"))?
    .run()
    .await
}
