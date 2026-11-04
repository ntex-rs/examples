# Askama templates

Render an HTML form and a personalized greeting with Askama. In `src/main.rs`,
`#[derive(Template)]` compiles the templates in `templates/`, and
`web::types::Query` extracts the optional `name` query parameter. The handler
calls `Template::render` and returns an HTML `HttpResponse`.

## Run the example

```sh
cd templates/askama
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| [`GET /`](http://localhost:8080/) | Name form from `templates/index.html` |
| [`GET /?name=Alice`](http://localhost:8080/?name=Alice) | Greeting `Hi, Alice!` and `Welcome!` from `templates/user.html` |

Open the form and submit a name, or request the greeting directly:

```sh
curl "http://localhost:8080/?name=Alice"
```

## Tests

```sh
cargo test -p template-askama
```

Covers the rendered name form, escaped greetings, and unknown routes.

## Learn more

- [Extractors and responders](https://docs.ntex.rs/8.4-web-app.html)
