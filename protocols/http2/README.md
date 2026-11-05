# HTTP/2: standalone server and client

This example shows how to:

- serve HTTP/2 with `ntex_h2::server::Server` and a publish service that
  keeps per-stream state keyed by `StreamRef`;
- collect a request body, and reject one that grows too large;
- stream a request body to another task together with its flow-control
  `Capacity`, so the client sends only as fast as the task consumes it;
- send a response with trailers;
- send requests with the pooled `client::Client`, waiting for send capacity
  before each chunk of a large body;
- configure HTTP/2 for every server connection.

| Binary | Source | What it does |
| --- | --- | --- |
| `http2-server` | [`src/main.rs`](src/main.rs) | Serves `/hello`, `/echo` and `/count` on `127.0.0.1:50060` |
| `http2-client` | [`src/client.rs`](src/client.rs) | Calls every route and prints the responses |

The publish service, the configuration and the shared client helpers are in
[`src/lib.rs`](src/lib.rs).

## Run it

Run every command below from this directory, `protocols/http2`.

Start the server in one terminal:

```sh
cargo run --bin http2-server
```

Then run the client from another terminal:

```sh
cargo run --bin http2-client
```

It prints:

```text
GET /hello   200 OK b"hello world"
POST /echo   200 OK b"hello again"
POST /count  200 OK b"counted 1048576 bytes" (x-bytes: 1048576)
```

The server speaks HTTP/2 over plain TCP, so other clients must use prior
knowledge. With curl:

```sh
curl --http2-prior-knowledge http://127.0.0.1:50060/hello
curl --http2-prior-knowledge --data 'hi there' http://127.0.0.1:50060/echo

# counts the body and shows the x-bytes trailer
head -c 1000000 /dev/zero | curl -sv --http2-prior-knowledge \
    --data-binary @- http://127.0.0.1:50060/count
```

## What to look for in the code

- **Publish service:** ntex-h2 creates a `Handler` for every connection and
  calls it with a `Message` for every event of every stream, concurrently.
  `Handler` keeps the requests whose body is still arriving in a map keyed
  by `StreamRef`, and removes them when the stream ends or fails.
- **Collecting a body:** `/echo` appends each data frame to a buffer and
  answers at the end of the stream. A body over `MAX_ECHO_BODY` gets a 413
  response right away, followed by a `NO_ERROR` reset that tells the client
  to stop sending.
- **Flow control:** every data frame comes with a `Capacity`. `/echo` drops
  it at once, so the client may keep sending. `/count` passes it, with the
  data, over a channel to a separate task, and the client can send more only
  after that task drops it.
- **Trailers:** the counting task answers with `send_response`,
  `send_payload` and finally `send_trailers`, which ends the stream.
- **Errors:** a reset or a lost connection reaches the handler as an `Eof`
  error or a `Disconnect` message. Removing the map entry then drops the
  channel sender, which ends the counting task.
- **Client:** `connect()` builds a pool for the server address. `send`
  returns a `SendStream` for the request body and a `RecvStream` for the
  response. The client reads the `/count` response while it is still
  sending, and waits on `send_capacity()` before each chunk.
- **Configuration:** `server_cfg()` adds `ntex_h2::ServiceConfig` to the
  listener configuration, and every connection reads it.

## Learn more

From the guide chapter [HTTP/2 with ntex-h2](https://docs.ntex.rs/14-h2.html):

- [Configuration](https://docs.ntex.rs/14-h2.html#configuration)
- [The client](https://docs.ntex.rs/14-h2.html#the-client)
- [Flow control](https://docs.ntex.rs/14-h2.html#flow-control)
- [The server](https://docs.ntex.rs/14-h2.html#the-server)

## Tests

```sh
cargo test -p http2
```

The tests serve `handler()` with `server_cfg()` and cover each route, a body
over the echo limit, and a body to `/count` larger than the connection's
flow-control window.
