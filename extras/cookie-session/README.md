# Cookie sessions

This example uses `ntex_session::CookieSession` to keep a visit counter in a
signed cookie. The `Session` extractor in `src/main.rs` reads and updates the
counter on each request to `/`. The response body stays `welcome!`; subsequent
visits print the previous counter value in the server console.

## Run the example

```sh
cd extras/cookie-session
cargo run
```

The server listens on `127.0.0.1:8080` and prints:

```text
Starting http server: 127.0.0.1:8080
```

The fixed signing key and `secure(false)` setting are for local demonstration
only. Signed cookies protect integrity, not confidentiality.

## Try it

Repeat this request, sending the saved session cookie back each time:

```sh
curl -b cookies.txt -c cookies.txt http://127.0.0.1:8080/
```

The first visit stores `counter = 1`. Later visits increment the counter and
print `SESSION value:` followed by the previous value. Remove `cookies.txt`
when finished.

## Tests

```sh
cargo test -p cookie-session
```

Covers signed session cookie replay and unknown-route responses.

## Learn more

- [Filters and Middleware](https://docs.ntex.rs/8.3-web-app.html#middleware)
