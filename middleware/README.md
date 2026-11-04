# Middleware

Four small middlewares that show the common patterns: inspecting the request
body, inspecting the response body, redirecting, and running code before and
after the handler.

## What is middleware?

A middleware is a service that wraps another service. It sits between the
server and the application handlers, so it sees every request before the
handler does and every response after the handler has produced it. This makes
it the right place for logic shared by many routes: logging, authentication,
default headers, compression, timeouts and so on.

A middleware has two parts:

- **Factory**, a type implementing `ntex::service::Middleware`. It is passed to
  `App::middleware()` and holds the middleware configuration.
- **Service**, a type implementing `ntex::service::Service`. The factory creates
  it once per worker thread with `Middleware::create()`, handing it the next
  service in the chain. The middleware service takes ownership of that service
  and handles every request in `Service::call()`.

In `call()` a middleware can:

- inspect or modify the request before passing it on,
- call the wrapped service with `ctx.call(&self.service, req)`,
- inspect or modify the response before returning it,
- or skip the wrapped service and return its own response.

Requests pass through middleware in the order it was added to the `App`,
responses travel back in the opposite order:

```text
App::new().middleware(A).middleware(B)

request  -> A -> B -> router -> handler
response <- A <- B <- router <- handler
```

`src/simple.rs` is a walkthrough of these steps.

## Run the example

```bash
cargo run -p middleware-example
```

The server listens on `http://127.0.0.1:8080`. To see what each middleware
does, comment its `.middleware(...)` line in `src/main.rs` in or out.

## Middlewares

### redirect::CheckLogin

A middleware implementing a request guard which sketches a rough approximation
of what a login could look like. It shows how a middleware can answer a request
itself, a redirect to `/login`, without calling the service it wraps, so the
rest of the chain never sees the request.

### read_request_body::Logging

A middleware demonstrating how to read out the incoming request body. It takes
the payload stream out of the request, reads it to the end, and puts the
collected bytes back, so the handler still receives the complete body. It also
shows how a middleware with errors of its own converts them into `WebError`.

### read_response_body::Logging

A middleware demonstrating how to read out the outgoing response body. A body
can be a stream produced while the response is sent, so the middleware replaces
it with a wrapper implementing `MessageBody`, which copies every chunk as it
passes to the client.

### simple::SayHi

A minimal middleware demonstrating the sequence of operations
in an ntex middleware: how it is created, how it wraps the next service, and how
it modifies the request and the response.

## Tests

```sh
cargo test -p middleware-example
```

Covers middleware headers, redirects, payload preservation and rejection, and unchanged response bodies without capturing logs.

## Learn more

- [Middleware](https://docs.ntex.rs/8.3-web-app.html#middleware)
- [Where filters and middleware run](https://docs.ntex.rs/8.3-web-app.html#where-filters-and-middleware-run)
- [Choosing between filters and middleware](https://docs.ntex.rs/8.3-web-app.html#choosing-between-them)
