# Yarte templates

Render a form or greeting with a compile-time, minified Yarte template.
In `src/main.rs`, `#[derive(TemplateMin)]` generates the renderer;
`TemplateMin::call` produces the HTML that the handler puts in an
`HttpResponse`. `yarte.toml` configures partial aliases, and `build.rs`
tracks template changes.

## Run the example

```sh
cd templates/yarte
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| [`GET /`](http://localhost:8080/) | Form asking for `name` and `lastname` |
| [`GET /?name=Alice&lastname=Smith`](http://localhost:8080/?name=Alice&lastname=Smith) | Greeting `Hi, Alice Smith!` and `Welcome` |
| [`GET /?name=Alice`](http://localhost:8080/?name=Alice) | `500 Internal Server Error` with body `Some error message`, because `lastname` is required when `name` is present |
| [`GET /?lastname=Smith`](http://localhost:8080/?lastname=Smith) | The form again, because `name` is absent |

The template and its partials live in `templates/`. They generate rendering
code, not an ntex responder implementation; the handler constructs the
response explicitly.

## Tests

From the repository root:

```sh
cargo test -p template_yarte
```

The test uses `web::test::init_service` without a network listener and checks
response status, content type, and rendered HTML for the cases above.

For broader context, see the [template benchmarks](https://github.com/botika/template-bench-rs).

## Learn more

- [Testing routes](https://docs.ntex.rs/8.1-web-app.html#testing-routes)
- [Returning errors from handlers](https://docs.ntex.rs/8.5-web-app.html#returning-errors-from-handlers)
