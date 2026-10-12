# State Accumulation

The state we have discussed so far is long-lived. Worker state lives for as long
as the worker, and pipeline state lives for as long as the pipeline. In both
cases, the state is created once and reused across many calls.

Sometimes we need state with a shorter lifetime. In particular, we may want to
collect information while a request or connection moves through a service
chain. Each service can inspect what has already been collected, add more
information, and pass the updated state to the next service.

Imagine a pipeline that accepts an incoming connection. The first service might
record basic details such as the connection ID and peer address. The next
service could read the TLS `ClientHello` and extract the Server Name Indication
(SNI). Other services might load configuration for that server name, validate
the client, calculate resource usage, apply throttling, and finally perform the
TLS handshake before handing the connection to an HTTP server.

The pipeline might look like this:

```text
accept connection
    → collect connection information
    → read SNI
    → load client information
    → validate and throttle
    → negotiate TLS
    → handle HTTP
```

ntex-service provides the `RequestState` trait and the `State` type for this
kind of state. Together, they let us pass a message and its accumulated state
through a service chain.

`State` and types that implement `RequestState` are different from pipeline state.
Pipeline state is shared across calls, while accumulated state belongs to
a single request. Every connection moving through the pipeline carries
its own state.

The ntex protocol servers support `RequestState`, including `ntex::http`,
ntex-h2, ntex-mqtt, and ntex-amqp.

## Collecting Connection Information

Let's start with a service that receives a raw I/O connection and collects some
basic information about it:

```rust
use std::{io, net::SocketAddr, time::Instant};
use ntex::{io::Io, service::State};
use uuid::Uuid;

struct Connection {
    id: Uuid,
    created: Instant,
    peer_addr: SocketAddr,
}

async fn connect(_: &AppState, io: Io) -> io::Result<State<Connection, Io>> {
    let peer_addr = load_peer_addr(&io)?;

    Ok(State {
        req: io,
        state: Connection {
            id: Uuid::new_v4(),
            created: Instant::now(),
            peer_addr,
        },
    })
}
```

The next service can unpack these values, perform the TLS handshake, and add
more information:

```rust
use ntex::service::{RequestState, State};

struct ConnectionWithTls {
    id: Uuid,
    created: Instant,
    peer_addr: SocketAddr,
    server_name: String,
}

async fn tls(_: &AppState, msg: State<Connection, Io>) -> io::Result<State<ConnectionWithTls, TlsIo>> {
    let (connection, io) = msg.unpack();

    let server_name = load_server_name(&io).await?;
    let io = accept_tls(io, &server_name).await?;

    Ok(State {
        req: io,
        state: ConnectionWithTls {
            id: connection.id,
            created: connection.created,
            peer_addr: connection.peer_addr,
            server_name,
        },
    })
}
```

This service changes both parts of the input. It turns the raw `Io` connection
into a TLS-enabled `TlsIo`, and it extends the connection state with the server
name extracted during TLS negotiation.

The connection ID, creation time, and peer address are carried forward. By the
time the connection reaches the HTTP server, its state contains everything
collected by the earlier services.

## Passing Connection State to the HTTP Service

We can now connect these services into a single chain:

```rust
use ntex::{server, service, SharedCfg};
use ntex::http::{self, Request, Response};

/// Handles an HTTP request using information about its connection.
async fn handle_request(st: &ConnectionWithTls, _req: Request) -> io::Result<Response> {
    Ok(Response::Ok()
        .body(format!("server name: {}", st.server_name)),
    )
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let builder = AppBuilder {
        // ...
    };

    server::build_with_config(builder)
        .bind(
            "HTTP",
            "127.0.0.1:8080",
            SharedCfg::new("S"),
            async |_: &AppState| {
                // Collect the initial connection information.
                service(connect)
                    // Perform the TLS handshake.
                    .and_then(tls)
                    // Pass the established connection to the HTTP service.
                    .and_then(http::HttpService::new(handle_request))
            },
        )?
        .run()
        .await
}
```

The worker's `AppState` is still available to `connect` and `tls`. It contains
worker-level resources that can be reused across many connections. At the same
time, each connection carries its own state.
