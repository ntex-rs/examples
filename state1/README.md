Process and Worker State
------------------------

See also the [Application state](https://ntex.rs/docs/application/#state),
[Shared mutable state](https://ntex.rs/docs/application/#shared-mutable-state) and
[Multi-threading](https://ntex.rs/docs/server/#multi-threading) sections of the
ntex guide.

Each ntex worker runs a single-threaded runtime. To use multiple CPU cores and
process connections concurrently, the networking server starts multiple
workers and distributes incoming connections among them. Each worker creates
and runs its own application instance.

Application initialization generally occurs in two stages. First, process-wide
configuration is loaded during startup, typically on the main thread. This may
include reading environment variables, parsing command-line arguments, loading
configuration files, or retrieving settings from an external configuration service.

The ntex server then uses this configuration to initialize an application instance
in each worker. This process is managed through the `ServerAppConfig` trait. The
server is initialized with an object that implements `ServerAppConfig`, shares that
object with its workers, and calls `ServerAppConfig::create()` inside each worker
to construct the worker-specific state.

The configuration object must implement `Send + Sync` because it is accessed
from multiple worker threads. The state returned by `create()`, however,
belongs to a single worker and therefore can use single-threaded types. This makes
it possible to use types such as `Rc`, `RefCell` or other values that do not
implement `Send` or `Sync` trait, provided they never leave the worker thread.

Because `create()` is asynchronous, it can also perform worker-specific
initialization. For example, it might establish database connections, create client
instances, initialize caches, or allocate other resources that should be
owned by an individual worker. If initialization fails, the method can return
an error and prevent that worker’s application instance from starting
with an invalid state.


```rust
struct AppBuilder {
    // Process-wide configuration
}

#[derive(Clone)]
struct AppState {
    // Worker-specific resources
}

impl ServerAppConfig for AppBuilder {
    type State = AppState;

    // Called separately within each worker thread.
    async fn create(&self) -> io::Result<Self::State> {
        Ok(AppState {
            // Initialize resources for this worker.
        })
    }
}
```

The state must implement `Clone`. ntex clones it when it hands the state to
the services of a worker, for example for every new connection. All clones
must therefore refer to the same data. Wrap mutable worker-local values in
`Rc` (`Rc<Cell<T>>`, `Rc<RefCell<T>>`) and process-wide values in `Arc`. A
plain `Cell<T>` field would be copied on every clone, and each connection
would end up with its own value instead of sharing the worker's.

```rust
#[derive(Clone)]
struct AppState {
    // Shared by all workers.
    global: Arc<AtomicUsize>,
    // Shared by all connections of one worker.
    local: Rc<Cell<usize>>,
}
```

The state returned by `create()` is also made available to the
connection-handler pipelines running in that worker. Consequently, state
is shared by connections handled by the same worker, but it is not
automatically shared across workers. Changes made to one worker’s state
are not visible to other workers.

If an application requires truly process-wide mutable state, it must be
shared explicitly—for example, through an `Arc` containing an appropriate
synchronization primitive. This distinction is important: worker-local
state avoids cross-thread synchronization and can improve performance,
while process-wide state provides coordination at the cost of additional
synchronization.

To initialize the application in each worker, the ntex server uses a factory
closure. This closure is called separately for every worker and receives
a reference to the worker-local state created by the configuration
object’s `ServerAppConfig::create()` method.

```rust
#[ntex::main]
async fn main() -> std::io::Result<()> {
    // Initialize the process-wide configuration.
    let builder = AppBuilder {
        // ...
    };

    web::server_with_config(builder, async |state: &AppState| {
        web::App::new()
            .service(
                web::resource("/").to(async || {
                    web::HttpResponse::Ok()
                }),
            )
    })
    .bind("127.0.0.1:8080", SharedCfg::default())?
    .run()
    .await
}
```

In this example, `AppBuilder` contains the process-wide configuration and
implements `ServerAppConfig`. For each worker, ntex calls `AppBuilder::create()`
to produce an `AppState`. It then passes a reference to that state to the
factory closure, which creates an independent `web::App` instance for the worker.

## Running the example

```bash
cargo run -p state-ch1
```

On startup, every worker prints the id assigned to it in `create()`:

```text
Creating state for worker 0
Creating state for worker 1
...
```

Send a few requests:

```bash
for i in $(seq 1 6); do curl http://127.0.0.1:8080/; done
```

Each response shows the worker that handled the request and three counters:

- `global mutex counter`: process-wide, `Arc<Mutex<usize>>`
- `global atomic counter`: process-wide, `Arc<AtomicUsize>`
- `local counter`: worker-local, `Rc<Cell<usize>>`

```text
worker: 1, global mutex counter: 1, global atomic counter: 1, local counter: 1
worker: 2, global mutex counter: 2, global atomic counter: 2, local counter: 1
worker: 3, global mutex counter: 3, global atomic counter: 3, local counter: 1
...
worker: 1, global mutex counter: 13, global atomic counter: 13, local counter: 2
```

The global counters grow with every request. Connections are distributed
among the workers, so each local counter grows only when its own worker
handles a request. Requests sent over a single keep-alive connection are
handled by the same worker:

```bash
curl http://127.0.0.1:8080/ http://127.0.0.1:8080/ http://127.0.0.1:8080/
```

Set `RUST_LOG=info` to see the request log produced by the `Logger`
middleware.
