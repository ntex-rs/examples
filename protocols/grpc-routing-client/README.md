# gRPC: routing client

This example shows how to:

- write a custom client transport by implementing `Transport` and
  `StreamTransport`;
- send every call to the backend of its service, using the request path of
  the generated method;
- share one transport between several generated clients;
- fail a call in the transport before anything is sent.

| Binary | Source | What it does |
| --- | --- | --- |
| `grpc-routing-client-server` | [`src/main.rs`](src/main.rs) | Runs the `shop.Catalog` backend on `127.0.0.1:50053` and the `shop.Orders` backend on `127.0.0.1:50054` |
| `grpc-routing-client` | [`src/client.rs`](src/client.rs) | Calls both services through one router and prints which backend answered |

The transport is in [`src/router.rs`](src/router.rs) and the two services in
[`src/lib.rs`](src/lib.rs). [`src/shop.rs`](src/shop.rs) is generated; don't
edit it.

## Before you start

- Run every command below from this directory, `protocols/grpc-routing-client`.
- You only need `protoc` and the code generator to regenerate the code after
  editing [`shop.proto`](shop.proto):

  ```sh
  cargo install ntex-grpc-codegen
  ./generate.sh
  ```

## Run it

Start both backends in one terminal:

```sh
cargo run --bin grpc-routing-client-server
```

Then run the client from another terminal:

```sh
cargo run --bin grpc-routing-client
```

It prints:

```text
GetItem    BOOK-42 "The Rust Programming Language" from catalog-1
ListItems  BOOK-42 "The Rust Programming Language" from catalog-1
ListItems  PEN-7 "Fountain pen" from catalog-1
ListItems  MUG-1 "Coffee mug" from catalog-1
PlaceOrder #1 from orders-1
```

The client only knows the two addresses. Each reply names the backend that
answered it.

## What to look for in the code

- **Routing key:** every generated method has a `MethodDef::PATH` such as
  `/shop.Catalog/GetItem`. `Router` takes the service name from it and looks
  up the backend, which is a regular `ntex_grpc::client::Client`.
- **Delegation:** after picking a backend, `Router` hands the call to the
  built-in `Client`, which applies headers and timeouts and maps errors. Only
  the built-in transports can open a `StreamCall`, so a custom
  `StreamTransport` always ends with one of them.
- **Request state:** both impls are generic over the request state `St`, so
  `send_with(&state)` and `open_with(&state)` go through the router too.
- **Sharing:** a generated client owns its transport. `Router` keeps its
  routes behind an `Rc`, so each typed client gets a cheap clone and every
  clone uses the same connection pools.
- **Errors:** a service without a route fails with
  `ClientError::InvalidRequest` and status `UNIMPLEMENTED`, without touching
  the network.

## Learn more

From the guide chapter [Building gRPC Services and Clients](https://docs.ntex.rs/11-grpc.html):

- [What happens during a call](https://docs.ntex.rs/11-grpc.html#what-happens-during-a-call)
- [Writing your own transport](https://docs.ntex.rs/11-grpc.html#writing-your-own-transport)
- [Routing calls to different backends](https://docs.ntex.rs/11-grpc.html#routing-calls-to-different-backends)

## Tests

```sh
cargo test -p grpc-routing-client
```

The tests start both backends, check that unary and streaming calls reach the
backend of their service, and that a call without a route fails with
`UNIMPLEMENTED`.
