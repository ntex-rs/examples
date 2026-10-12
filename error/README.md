Error handling
--------------

Shows how to define a custom error domain and render every error the
application returns, including errors of ntex extractors, as a uniform JSON
document. See `src/main.rs` for detailed comments.

See also the [Errors](https://ntex.rs/docs/errors/) and
[Application state](https://ntex.rs/docs/application/) sections of the ntex
guide.

## What it shows

- **Error domain.** `AppState` implements `web::State` with
  `type Error = AppErrors`. Handlers, extractors and services of the
  application must return errors that implement
  `WebResponseError<AppState, AppErrors>`. This is checked at compile time.
- **Custom renderers.** `CustomError` maps each variant to a status code. The
  state is passed to `error_response()`, so renderers can use it (here, to add
  the service name).
- **Rendering ntex errors.** ntex renders its own errors only in the default
  domain (`web::DefaultError`). With a custom domain the application provides
  the renderer, here for the `Json<T>` extractor's `JsonPayloadError`.
- **Error container.** `AppError` boxes any error of the domain and keeps its
  renderer, so handlers can use `?` with different error types.
- **Hiding internals.** The `500` response only says "internal server error";
  the real cause is written to the log.

## Running the example

```bash
cargo run -p error_handling
```

The server listens on `127.0.0.1:8088`. Requests and failed calls are logged to
the console; set `RUST_LOG` to change the log level.

## Endpoints

`POST /something` expects a JSON body such as `{"dummy": 1}`. The handler
succeeds with probability 1/5. Otherwise it fails with a random error:

| Error                           | Response                    |
|---------------------------------|-----------------------------|
| `CustomError::AccessDenied`     | `403 Forbidden`             |
| `CustomError::NotAuthenticated` | `401 Unauthorized`          |
| `CustomError::ServiceFailure`   | `500 Internal Server Error` |
| `CustomError::InvalidInput`     | `400 Bad Request`           |
| invalid or missing JSON body    | `400 Bad Request`           |

```bash
# repeat a few times to see different errors
curl -i -H 'content-type: application/json' -d '{"dummy": 1}' http://127.0.0.1:8088/something
# HTTP/1.1 403 Forbidden
# content-type: application/json
#
# {"service":"error-example","status":403,"error":"access to this resource is denied"}

# a malformed body is rejected by the extractor before the handler runs
curl -H 'content-type: application/json' -d '{bad' http://127.0.0.1:8088/something
# {"service":"error-example","status":400,"error":"Json deserialize error: key must be a string at line 1 column 2"}
```
