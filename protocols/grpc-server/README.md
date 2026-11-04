# gRPC: server

This example shows how to:

- implement each kind of gRPC method with `#[server]`: unary, server
  streaming, client streaming and bidirectional streaming;
- read request headers in a handler;
- end a streaming call with an error status;
- serve several services from one `GrpcServer` and share state between
  them;
- answer calls for unknown services with a hand-written default service;
- limit the request size and configure HTTP/2 for every connection.

| Binary | Source | What it does |
| --- | --- | --- |
| `grpc-server` | [`src/main.rs`](src/main.rs) | Serves `echo.Echo` and `echo.Stats` on `127.0.0.1:50055` |
| `grpc-server-client` | [`src/client.rs`](src/client.rs) | Calls every `echo.Echo` method, then `echo.Stats/GetStats` |

The services, the default service and the configuration are in
[`src/lib.rs`](src/lib.rs). [`src/echo.rs`](src/echo.rs) is generated; don't
edit it.

## Before you start

- Run every command below from this directory, `protocols/grpc-server`.
- You only need `protoc` and the code generator to regenerate the code after
  editing [`echo.proto`](echo.proto):

  ```sh
  cargo install ntex-grpc-codegen
  ./generate.sh
  ```

## Run it

Start the server in one terminal:

```sh
cargo run --bin grpc-server
```

Then run the client from another terminal:

```sh
cargo run --bin grpc-server-client
```

It prints:

```text
UnaryEcho           > hello
ServerStreamingEcho again
ServerStreamingEcho again
ServerStreamingEcho again
ClientStreamingEcho one two three (3 messages)
BidiStreamingEcho   ping
BidiStreamingEcho   pong
GetStats            4 calls
```

Any gRPC client works too. With
[grpcurl](https://github.com/fullstorydev/grpcurl):

```sh
grpcurl -plaintext -import-path . -proto echo.proto \
    -d '{"message": "hi", "repeat": 2}' \
    127.0.0.1:50055 echo.Echo/ServerStreamingEcho

# fails with InvalidArgument: message must not be empty
grpcurl -plaintext -import-path . -proto echo.proto \
    -d '{"message": ""}' \
    127.0.0.1:50055 echo.Echo/UnaryEcho

# fails with InvalidArgument: repeat must be at most 100
grpcurl -plaintext -import-path . -proto echo.proto \
    -d '{"message": "hi", "repeat": 500}' \
    127.0.0.1:50055 echo.Echo/ServerStreamingEcho

# fails with DeadlineExceeded
grpcurl -plaintext -import-path . -proto echo.proto -max-time 0.2 \
    -d '{"message": "slow", "delay_ms": 1000}' \
    127.0.0.1:50055 echo.Echo/UnaryEcho
```

## What to look for in the code

- **Handler signatures:** `#[server]` picks the kind of each method from its
  arguments. A request message, or `Request<_>` for the headers too, makes a
  unary or server-streaming call. A `RecvStream<_>` makes a client-streaming
  or bidirectional call. Streaming replies go to a `SendStream<_>`.
- **Errors:** a handler fails the call by returning a `ServerError`, and
  the error becomes the status and message of the call. Unary and
  client-streaming handlers return `Result<Response<_>, ServerError>`,
  streaming handlers `Result<(), ServerError>`. `?` works on `StreamError`
  too.
- **Trailers:** a unary or client-streaming handler that returns
  `Response<_>` sends its `headers` as trailers after `grpc-status`.
  `ClientStreamingEcho` sends `x-message-count`, and the client reads it
  with `trailers()`.
- **Several services:** `GrpcServer::new()` takes the first service and
  `.service()` adds more. Calls are routed by the service name in their
  path. `GrpcServer` creates the services for every connection, so
  `EchoServer` and `StatsServer` share their counter through an `Arc`.
- **Default service:** `Fallback` implements `Service` by hand. It gets the
  raw `ServerRequest`, with the service and method names, and answers
  `UNIMPLEMENTED` with a helpful message.
- **Limits:** `.max_message_size()` rejects larger requests with
  `RESOURCE_EXHAUSTED`. The client timeout is sent as `grpc-timeout`, and
  the server stops the handler once it passes.
- **Configuration:** `server_cfg()` adds `ntex_h2::ServiceConfig` to the
  listener configuration. `GrpcServer` reads it from every connection.

## Learn more

From the guide chapter [Building gRPC Services and Clients](https://docs.ntex.rs/11-grpc.html):

- [Implementing a service](https://docs.ntex.rs/11-grpc.html#implementing-a-service)
- [Tuning the server](https://docs.ntex.rs/11-grpc.html#tuning-the-server)
- [Errors, deadlines and retries](https://docs.ntex.rs/11-grpc.html#errors-deadlines-and-retries)

## Tests

```sh
cargo test -p grpc-server
```

The tests serve `grpc_server()` with `server_cfg()` and cover each kind of
method, a streaming error status, the shared call counter, a deadline, the
message size limit and the default service.
