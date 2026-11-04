# Nested resource configuration

This example registers product and part routes through `App::configure`.
Look at `config_app` in `src/appconfig.rs` for nested `web::scope` calls, and
`src/handlers/` for JSON, query, and path extractors. The handlers are stubs;
there is no database or product storage.

## Run the example

```sh
cd extras/async-ex2
cargo run --bin main
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| `GET /products` | Product-list handler |
| `POST /products` | Product-creation handler; accepts JSON |
| `GET /products/{product_id}` | Product-detail handler |
| `DELETE /products/{product_id}` | Product-removal handler |
| `GET /products/{product_id}/parts` | Part-list handler |
| `POST /products/{product_id}/parts` | Part-creation handler; accepts JSON |
| `GET /products/{product_id}/parts/{part_id}` | Part-detail route; see limitation below |
| `DELETE /products/{product_id}/parts/{part_id}` | Part-removal route; see limitation below |

Handlers return empty `200 OK` responses when extraction succeeds. The two
part-detail routes currently use `Path<String>` even though their paths have
two captures, so extraction fails before those handlers run.

```sh
curl -i -X POST http://127.0.0.1:8080/products \
  -H 'Content-Type: application/json' \
  -d '{"id":12345,"product_type":"fancy","name":"test"}'
```

## Tests

```sh
cargo test -p async-ex2
```

Covers product creation, malformed JSON rejection, and nested product-detail routing without a database.

## Learn more

- [Scopes](https://docs.ntex.rs/8.1-web-app.html#scopes)
- [Modular configuration](https://docs.ntex.rs/8.1-web-app.html#modular-configuration)
