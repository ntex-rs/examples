# Form extraction

Deserialize URL-encoded form submissions with `web::types::Form<MyParams>`.
The handlers in `src/main.rs` show the extractor on its own, alongside
application state with `to_with_state`, and alongside an `HttpRequest`.
The HTML page in `static/form.html` provides a form for each handler.

## Run the example

```sh
cd extractors/form
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

Open <http://127.0.0.1:8080/> to submit the forms in a browser. Each POST
expects an `application/x-www-form-urlencoded` body with a `name` field.

| Request | Result |
| --- | --- |
| `GET /` | Serve the HTML forms |
| `POST /post1` | Return the submitted name as plain text |
| `POST /post2` | Return the name and the `foo` value (`bar`) from `AppState` |
| `POST /post3` | Print the request debug representation and return the name as plain text |

```sh
curl -d "name=John" http://127.0.0.1:8080/post1
curl -d "name=John" http://127.0.0.1:8080/post2
curl -d "name=John" http://127.0.0.1:8080/post3
```

`/post1` and `/post3` return `Your name is John`; `/post2` returns
`Your name is John, and in AppState I have foo: bar`.

## Tests

```sh
cargo test -p form-example
```

Covers the HTML form, all three form handlers, application state, and rejection of missing names.

## Learn more

- [Extractors](https://docs.ntex.rs/8.4-web-app.html#extractors)
- [Accessing application state](https://docs.ntex.rs/8.2-web-app.html#accessing-application-state-in-handlers)
