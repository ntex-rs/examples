# Tera templates

Render a name form and greeting with Tera. In `src/main.rs`, each worker
loads `templates/` with `Tera::load_from_glob` and stores the engine in
`web::AppState`. The handler extracts query parameters, renders a template,
and maps rendering failures to `500 Internal Server Error`.

## Run the example

```sh
cd templates/tera
cargo run
```

The server listens on `127.0.0.1:8080`. The template glob uses
`CARGO_MANIFEST_DIR`, so templates are resolved from the crate directory
rather than the process working directory.

## Try it

| Request | Result |
| --- | --- |
| [`GET /`](http://localhost:8080/) | Name form from `templates/index.html` |
| [`GET /?name=Alice`](http://localhost:8080/?name=Alice) | Greeting `Hi, Alice!` and `Welcome!` from `templates/user.html` |

```sh
curl "http://localhost:8080/?name=Alice"
```

## Tests

```sh
cargo test -p template-tera
```

Covers the rendered form, escaped greetings, and template-rendering failures.

## Learn more

- [Accessing application state in handlers](https://docs.ntex.rs/8.2-web-app.html#accessing-application-state-in-handlers)
