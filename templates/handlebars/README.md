# Handlebars templates

Render HTML with Handlebars and shared application state. `src/main.rs`
registers the `.html` templates in `static/templates/` once and shares the
registry with workers through `Arc<Handlebars>` in `web::AppState`.
The handlers use `Handlebars::render` with JSON data and return an
`HttpResponse` body.

## Run the example

```sh
cd templates/handlebars
cargo run
```

The server listens on `127.0.0.1:8080`. Templates are resolved from the crate
directory using `CARGO_MANIFEST_DIR`, independently of the working directory.

## Try it

| Request | Result |
| --- | --- |
| [`GET /`](http://localhost:8080/) | Renders `index.html` with `name` set to `Handlebars` |
| [`GET /Emma/documents`](http://localhost:8080/Emma/documents) | Renders `user.html` with `user=Emma` and `data=documents` |
| [`GET /Bob/passwords`](http://localhost:8080/Bob/passwords) | Renders the same template with `user=Bob` and `data=passwords` |

The dynamic route is `/{user}/{data}`. These values are only displayed in
the template; the example does not read documents or passwords.

## Tests

```sh
cargo test -p template_handlebars
```

Covers the real template registry, rendered index and user pages, HTML escaping, and incomplete routes.

## Learn more

- [Worker-local and process-wide state](https://docs.ntex.rs/6-server-app.html#worker-local-and-process-wide-state)
- [Accessing path variables](https://docs.ntex.rs/8.1-web-app.html#accessing-path-variables)
