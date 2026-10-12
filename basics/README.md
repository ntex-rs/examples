Basics
------

A tour of the basic building blocks of an ntex web application: route macros
and resources, path parameters, cookie sessions, streaming responses, static
files, redirects, errors and a custom 404 page. See `src/main.rs` for detailed
comments.

See also the [Request handlers](https://ntex.rs/docs/handlers/),
[URL dispatch](https://ntex.rs/docs/url-dispatch/),
[Static files](https://ntex.rs/docs/static-files/),
[Errors](https://ntex.rs/docs/errors/) and
[User sessions](https://ntex.rs/docs/middleware/#user-sessions) sections of
the ntex guide.

## Running the example

```bash
cargo run -p basics
```

The server listens on `127.0.0.1:8080`. Static files are resolved relative to
the crate directory, so the example can be started from any directory. Requests
are logged to the console; set `RUST_LOG` to change the log level.

## Endpoints

| Request                                                     | Result                                                                 |
|-------------------------------------------------------------|------------------------------------------------------------------------|
| [`GET /`](http://localhost:8080/)                           | `302 Found` redirect to `/welcome`                                     |
| [`GET /welcome`](http://localhost:8080/welcome)             | HTML page with a visit counter stored in a cookie session              |
| [`GET /favicon`](http://localhost:8080/favicon)             | `static/favicon.ico`, served with `NamedFile`                          |
| [`GET /static/`](http://localhost:8080/static/)             | Directory listing of `static/`; files are served from `/static/<file>` |
| [`GET /user/bob`](http://localhost:8080/user/bob)           | `Hello bob!` — path parameter extracted with `Path<T>`                 |
| [`GET /files/a/b/c`](http://localhost:8080/files/a/b/c)     | `matched tail: a/b/c` — a tail segment matches the rest of the path    |
| [`GET /async-body/bob`](http://localhost:8080/async-body/bob) | `Hello bob!` sent as a streaming (chunked) body                      |
| `GET /test`, `POST /test`, other methods                    | `200`, `405` and `404` — the handler matches on the method             |
| [`GET /error`](http://localhost:8080/error)                 | `500 Internal Server Error` built with `InternalError`                 |
| any other `GET` request                                     | custom 404 page (`static/404.html`)                                    |
| any other non-`GET` request                                 | `405 Method Not Allowed`                                               |

Try the session counter and the method-based routes with curl:

```bash
# the counter increases on each request that sends the session cookie back
curl -c cookies.txt -b cookies.txt http://localhost:8080/welcome

curl -i -X POST http://localhost:8080/test       # 405 Method Not Allowed
curl -i -X DELETE http://localhost:8080/test     # 404 Not Found
curl -i -X POST http://localhost:8080/not-found  # 405 from the default service
```
