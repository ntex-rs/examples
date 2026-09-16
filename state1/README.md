Process and Worker State
------------------------

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
from multiple worker threads. The state returned by  create() , however,
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
