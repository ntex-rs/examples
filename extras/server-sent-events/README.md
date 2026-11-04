# Server-sent events

This example broadcasts server-sent events to browser `EventSource` clients.
In `src/main.rs`, `HttpResponse::streaming` wraps a Tokio channel receiver,
and a shared `Broadcaster` tracks clients and sends periodic keepalive events.
The browser page in `src/index.html` timestamps each received message.

## Run the example

```sh
cd extras/server-sent-events
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

Open <http://127.0.0.1:8080/> in a browser, then broadcast a message:

```sh
curl http://127.0.0.1:8080/broadcast/my_message
```

The response is `msg sent`, and `my_message` appears in the browser with a
timestamp. To watch the event stream directly, use another terminal:

```sh
curl -N http://127.0.0.1:8080/events
```

| Request | Result |
| --- | --- |
| `GET /` | Browser client page |
| `GET /events` | `text/event-stream` connection, starting with `data: connected` |
| `GET /broadcast/{msg}` | Broadcasts `msg` to connected clients |

## Benchmark tools

With Node.js installed, run these scripts from `extras/server-sent-events`
against the running server:

```sh
node benchmark.js
node drain.js
```

[`benchmark.js`](benchmark.js) opens 1,000 clients and reports connection and
broadcast timings. [`drain.js`](drain.js) repeatedly opens and drops connections
while probing whether the server is accepting requests. Results depend on
your machine; interrupt each script when finished.

If you encounter “Too many open files”, your shell's file-descriptor limit
may be too low. Where permitted, raise it before starting the server and
clients, for example:

```sh
ulimit -n 2048
```

The drain script's `accepting connections` value indicates whether its latest
probe received a response before the timeout; it is not a diagnosis of
resource exhaustion.

## Tests

```sh
cargo test -p server-sent-events
```

Covers the browser page, the first SSE event, broadcast delivery, and unknown routes without waiting for keepalives.

## Learn more

- [Responders](https://docs.ntex.rs/8.4-web-app.html#responders)
