# gRPC: hello world

This example shows how to:

- generate messages, a service definition and a typed client from a `.proto`
  file with `ntex-grpc-codegen`;
- implement the service with the `#[server]` attribute and serve it with
  `GrpcServer`;
- call it with the generated client over an ntex-h2 connection pool.

| Binary | Source | What it does |
| --- | --- | --- |
| `grpc-helloworld-server` | [`src/main.rs`](src/main.rs) | Serves `helloworld.Greeter` on `127.0.0.1:50051` |
| `grpc-helloworld-client` | [`src/client.rs`](src/client.rs) | Calls `SayHello` and prints the reply |

The service and the client helper live in [`src/lib.rs`](src/lib.rs).
[`src/helloworld.rs`](src/helloworld.rs) is generated; don't edit it.

## Before you start

- Run every command below from this directory, `protocols/grpc-helloworld`.
- You only need `protoc` and the code generator to regenerate
  `src/helloworld.rs` after editing [`helloworld.proto`](helloworld.proto):

  ```sh
  cargo install ntex-grpc-codegen
  ./generate.sh
  ```

## Run it

Start the server in one terminal:

```sh
cargo run --bin grpc-helloworld-server
```

Then call it from another:

```sh
cargo run --bin grpc-helloworld-client -- ntex
```

You should see `Hello ntex!`. Without a name the server answers
`Hello world!`.

Any gRPC client works too. For example, with
[grpcurl](https://github.com/fullstorydev/grpcurl):

```sh
grpcurl -plaintext -proto helloworld.proto -d '{"name": "ntex"}' \
    127.0.0.1:50051 helloworld.Greeter/SayHello
```

## What to look for in the code

- **Generated code:** plain structs for the messages, `Greeter` as the service
  definition, one `MethodDef` type per method and `GreeterClient<T>`. No
  derive or build script is involved; the file is committed.
- **Server:** `#[server(crate::helloworld::Greeter)]` maps the `SayHello`
  method to `say_hello`. `GreeterServer` also implements `ServiceFactory`,
  because `GrpcServer` creates a service for every connection.
- **Client:** `connect` wraps an ntex-h2 client pool in
  `ntex_grpc::client::Client` and passes it to `GreeterClient::new`. The pool
  connects on the first call. `say_hello` returns a request builder, which
  sets the deadline before `send()`.

## Learn more

From the guide chapter [Building gRPC Services and Clients](https://docs.ntex.rs/11-grpc.html):

- [From .proto to Rust](https://docs.ntex.rs/11-grpc.html#from-proto-to-rust) with `ntex-grpc-codegen`
- [Connecting to a server](https://docs.ntex.rs/11-grpc.html#connecting-to-a-server) and
  [what happens during a call](https://docs.ntex.rs/11-grpc.html#what-happens-during-a-call)
- [Implementing a service](https://docs.ntex.rs/11-grpc.html#implementing-a-service) with `#[server]`

## Tests

```sh
cargo test -p grpc-helloworld
```

The tests start the server on a test port and call it with the generated client, with and without a name.
