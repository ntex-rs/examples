# Cookie-based identity

This example uses `ntex-identity`: `IdentityService` installs a
`CookieIdentityPolicy`, and handlers use `Identity` to remember or forget a
user. Login always selects `user1`; it does not check credentials.

## Run the example

```sh
cd extras/cookie-auth
cargo run
```

The server listens on `127.0.0.1:8080`. Its fixed key and
`secure(false)` cookies are for local demonstration only.

## Try it

Use a cookie jar instead of copying an encoded cookie value:

```sh
curl http://127.0.0.1:8080/
curl -i -c cookies.txt -X POST http://127.0.0.1:8080/login
curl -b cookies.txt http://127.0.0.1:8080/
curl -i -b cookies.txt -c cookies.txt http://127.0.0.1:8080/logout
curl -b cookies.txt http://127.0.0.1:8080/
```

| Request | Result |
| --- | --- |
| `GET /` | `Hello Anonymous`, or `Hello user1` with a valid identity cookie |
| `POST /login` | Remembers `user1` in the `auth-example` cookie and redirects to `/` |
| `GET /logout` | Forgets the identity and redirects to `/` |

The cookie is encoded by the middleware; any displayed cookie value would be
an example, not a fixed value to reuse. Remove `cookies.txt` when finished.

## Tests

```sh
cargo test -p cookie-auth
```

Covers anonymous visitors, login cookie replay, logout cookie removal, and forged cookies.

## Learn more

- [Filters and Middleware](https://docs.ntex.rs/8.3-web-app.html#middleware)
