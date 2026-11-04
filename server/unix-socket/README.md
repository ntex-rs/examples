# HTTP over a Unix domain socket

Serve an ntex web application through `web::HttpServer::bind_uds` instead
of a TCP listener. `src/main.rs` registers two greeting routes and uses the
normal server worker pool; the default worker count follows the available
logical CPUs, not a single worker.

## Prerequisites

A Unix platform and a curl build with Unix domain socket support. On other
platforms, this example only prints `not supported` and exits.

## Run the example

```sh
cd server/unix-socket
cargo run
```

The server listens on the Unix socket `/tmp/ntex-uds.socket`, not a TCP
address. ntex removes an existing file at that path before binding and
attempts to remove the socket file when the server stops normally. A forced
process termination may leave it behind; manual deletion is not normally
needed before restarting. Do not use this path for an unrelated file or
another running server.

## Try it

```sh
curl --unix-socket /tmp/ntex-uds.socket http://localhost/
curl --unix-socket /tmp/ntex-uds.socket http://localhost/index.html
```

Both requests return `Hello world!`. Here `localhost` supplies the HTTP
host; curl connects through the socket rather than a TCP port.

## Learn more

- [Binding and existing listeners](https://docs.ntex.rs/5-server.html#binding-and-existing-listeners)
- [Workers and service factories](https://docs.ntex.rs/5-server.html#workers-and-service-factories)

## Tests

```sh
cargo test -p unix-socket
```

Covers greeting routes, unknown routes, and HTTP over a unique Unix socket that is cleaned up; tests run only on Unix.
