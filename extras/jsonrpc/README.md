# JSON-RPC over HTTP

This example dispatches JSON-RPC-style requests through `POST /`. Look at
`rpc_handler` and `rpc_select` in `src/main.rs` for dispatch, and
`src/convention.rs` for request and response types. An `Arc<RpcState>` shares
a counter between workers, with an `RwLock` protecting access.

## Run the example

```sh
cd extras/jsonrpc
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

```sh
curl -X POST http://127.0.0.1:8080/ \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"ping","params":[],"id":1}'
```

The `ping` response is:

```json
{"jsonrpc":"2.0","result":"pong","error":null,"id":1}
```

| Method | `params` | Result |
| --- | --- | --- |
| `ping` | `[]` | `"pong"` immediately |
| `wait` | `[n]`, where `n` is an unsigned integer | `"pong"` after `n` seconds |
| `get` | `[]` | Current counter value, initially `0` |
| `inc` | `[]` | Increments the counter and returns `null` |

This is a small demonstration, not a complete JSON-RPC 2.0 implementation.
Requests use positional `params`, and responses always serialize both
`result` and `error`, including the null field. RPC errors also use HTTP
`200 OK`; their `error` object contains `code`, `message`, and `data`.

The existing Python client exercises all four methods. With Python 3 and the
`requests` package installed, run it from `extras/jsonrpc` while the server
is running:

```sh
python3 -m pip install requests
python3 tests/test_client.py
```

## Tests

```sh
cargo test -p jsonrpc-example
```

Covers successful RPC methods and shared counter state, unknown methods, malformed requests, and invalid wait parameters.

## Learn more

- [Application State](https://docs.ntex.rs/8.2-web-app.html#accessing-application-state-in-handlers)
