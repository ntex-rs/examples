//! Process-wide state vs. worker-local state.
//!
//! By default, `ntex` starts one worker per logical CPU core, and every worker
//! runs its own `App` instance. This example keeps three counters:
//!
//! * `counter1` - process-wide state protected by a `Mutex`.
//! * `counter2` - process-wide state stored in an `AtomicUsize`.
//! * `counter3` - worker-local state stored in an `Rc<Cell<_>>`.
//!
//! `counter1` and `counter2` are shared by all workers, so they are incremented
//! on every request. Each worker has its own `counter3`, so with `N` workers it
//! counts only the requests handled by that worker (roughly one in `N`).
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::{cell::Cell, io, rc::Rc};

use ntex::web::{self, App, HttpRequest, HttpResponse, middleware};
use ntex::{SharedCfg, server::ServerAppConfig};

/// Worker-local application state.
///
/// One value is created per worker by `AppBuilder::create()`. ntex clones the
/// state when it passes it to the services of a worker (for example, for
/// every new connection), so the state must implement `Clone`.
///
/// Cloning must not split state that is meant to be shared. Every field is
/// therefore a handle: `Arc` for process-wide values and `Rc` for values that
/// belong to this worker. A bare `Cell<usize>` would be copied on clone, and
/// each connection would get its own counter.
#[derive(Clone)]
struct AppState {
    // Index of the worker that owns this state.
    worker_id: usize,
    // Process-wide counters, shared by all workers.
    counter1: Arc<Mutex<usize>>,
    counter2: Arc<AtomicUsize>,
    // Worker-local counter. `Rc` and `Cell` are neither `Send` nor `Sync`,
    // which is fine because this value never leaves the worker thread.
    counter3: Rc<Cell<usize>>,
}

/// Enables `AppState` to be used as `web::App` state.
impl web::State for AppState {
    type Error = web::DefaultError;
}

/// Process-wide configuration.
///
/// The server shares this value with all worker threads, so it must be
/// `Send + Sync`. It holds everything that is common to the whole process.
struct AppBuilder {
    counter1: Arc<Mutex<usize>>,
    counter2: Arc<AtomicUsize>,
    // Used to give every worker a distinct id.
    next_worker_id: AtomicUsize,
}

impl AppBuilder {
    fn new() -> Self {
        Self {
            counter1: Arc::new(Mutex::new(0)),
            counter2: Arc::new(AtomicUsize::new(0)),
            next_worker_id: AtomicUsize::new(0),
        }
    }
}

impl ServerAppConfig for AppBuilder {
    type State = AppState;

    /// Creates the state for one server worker.
    ///
    /// Called once inside each worker thread before the worker starts
    /// accepting connections. It is async, so it can also open database
    /// connections, create clients or warm caches owned by the worker.
    /// Returning an error prevents the worker from starting.
    async fn create(&self) -> io::Result<Self::State> {
        let worker_id = self.next_worker_id.fetch_add(1, Ordering::Relaxed);
        println!("Creating state for worker {worker_id}");

        Ok(AppState {
            worker_id,
            // Clone the handles to the process-wide counters.
            counter1: self.counter1.clone(),
            counter2: self.counter2.clone(),
            // Create a counter owned by this worker.
            counter3: Rc::new(Cell::new(0)),
        })
    }
}

/// Updates and shows the process-wide and worker-local counters.
///
/// `to_with_state()` passes a reference to the worker state as the
/// first argument.
async fn index(st: &AppState, _: (), _req: HttpRequest) -> HttpResponse {
    // Process-wide: every access goes through a lock.
    let counter1 = {
        let mut counter = st.counter1.lock().unwrap();
        *counter += 1;
        *counter
    };

    // Process-wide: lock-free, but still synchronized between threads.
    let counter2 = st.counter2.fetch_add(1, Ordering::Relaxed) + 1;

    // Worker-local: no synchronization is needed.
    let counter3 = st.counter3.get() + 1;
    st.counter3.set(counter3);

    HttpResponse::Ok().body(format!(
        "worker: {}, global mutex counter: {counter1}, global atomic counter: {counter2}, local counter: {counter3}\n",
        st.worker_id
    ))
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    // Create the process-wide configuration before building the server.
    let cfg = AppBuilder::new();

    // Start the server with the application configuration. ntex calls
    // `AppBuilder::create()` in every worker, then calls the factory closure
    // with a reference to the resulting worker state. The `App` built by the
    // closure uses that state.
    web::server_with_config(cfg, async |_: &AppState| {
        App::new()
            // Enable request logging (set RUST_LOG=info to see it).
            .middleware(middleware::Logger::default())
            // Register the request handler.
            .service(web::resource("/").to_with_state(index))
    })
    .bind("127.0.0.1:8080", SharedCfg::new("STATE"))?
    .run()
    .await
}
