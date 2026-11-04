# Custom JSON error responses

This example implements `web::WebResponseError` for a custom `Error` type.
Look at `error_response` in `src/main.rs`: it chooses an HTTP status and uses
`HttpResponse::json` to build the error body independently of the error's
`Display` implementation.

## Run the example

```sh
cd extras/json-error
cargo run
```

The server listens on `127.0.0.1:8000`.

## Try it

```sh
curl -i http://127.0.0.1:8000/
```

`GET /` always returns `400 Bad Request` with this JSON body:

```json
{"error":"an example error message"}
```

## Tests

```sh
cargo test -p json-error
```

Covers the custom JSON error response and routing to an unknown path.

## Learn more

- [Error Handling](https://docs.ntex.rs/8.5-web-app.html#returning-errors-from-handlers)
