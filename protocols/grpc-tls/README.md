# gRPC: TLS

This example shows how to:

- serve gRPC over TLS by running rustls's `TlsAcceptor` before `GrpcServer`;
- offer HTTP/2 with ALPN on both sides;
- give the h2 client pool a `TlsConnector` so generated clients call over
  TLS.

It reuses the `helloworld.Greeter` service from
[`grpc-helloworld`](../grpc-helloworld), so only TLS is new here.

| Binary | Source | What it does |
| --- | --- | --- |
| `grpc-tls-server` | [`src/main.rs`](src/main.rs) | Serves `helloworld.Greeter` with TLS on `127.0.0.1:50056` |
| `grpc-tls-client` | [`src/client.rs`](src/client.rs) | Calls `SayHello` over TLS, verifying the server against the local CA |

The TLS configuration, the server pipeline and the client are in
[`src/lib.rs`](src/lib.rs).

## Before you start

- No system TLS library is needed. rustls and its ring crypto provider are
  pure Rust dependencies.
- Create the [local certificates](../../tls/README.md#step-1-create-local-certificates)
  with `../../tls/generate-certs.sh`. The server loads
  `../../tls/certs/server.pem` and `server.key`, and the client trusts
  `../../tls/certs/ca.pem`.
- Run every command below from this directory, `protocols/grpc-tls`.

## Run it

Start the server in one terminal:

```sh
cargo run --bin grpc-tls-server
```

Then run the client from another terminal:

```sh
cargo run --bin grpc-tls-client -- ntex
```

It prints:

```text
Hello ntex!
```

The client connects to `localhost:50056`, because the server certificate is
issued for `localhost`.

With [grpcurl](https://github.com/fullstorydev/grpcurl):

```sh
grpcurl -cacert ../../tls/certs/ca.pem \
    -import-path ../grpc-helloworld -proto helloworld.proto \
    -d '{"name": "curl"}' localhost:50056 helloworld.Greeter/SayHello
```

Without `-cacert`, or with `-plaintext`, the call fails.

## What to look for in the code

- **Server pipeline:** `grpc_tls_service()` chains `TlsAcceptor` into
  `grpc_server()` with `and_then`. `GrpcServer` serves any `Io`, so it gets
  the TLS stream as is. Nothing in the gRPC service changes.
- **ALPN:** gRPC runs over HTTP/2, so `server_config()` and
  `client_config()` both offer only `h2`.
- **Client:** `connect()` replaces the h2 pool's TCP connector with
  `TlsConnector`. `secure(true)` makes the client send the `https` scheme.
  The generated `GreeterClient` is the same as in the plaintext example.
- **Crypto provider:** both configurations pass ring to
  `builder_with_provider`, so no process-wide default is installed.

## Learn more

- [Running over TLS](https://docs.ntex.rs/11-grpc.html#running-over-tls)
  in the guide chapter [Building gRPC Services and Clients](https://docs.ntex.rs/11-grpc.html)
- The guide chapter [Securing Connections with TLS](https://docs.ntex.rs/10-tls.html)

## Tests

```sh
cargo test -p grpc-tls
```

The tests generate certificates in memory, so they don't need
`generate-certs.sh`. They check that a trusted client gets a reply, and that
a client trusting another CA and a plaintext client both fail.
