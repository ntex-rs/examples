# Invitation-based authentication

This example combines `ntex-identity` cookies, PostgreSQL, and invitation
emails. `web::scope` groups the API under `/api` in `src/main.rs`; the
handler modules use `web::block` for Diesel queries. `argonautica` hashes
passwords, `sparkpost` sends email, and `lazy_static` initializes configuration.

The flow is: request an email invitation, register a password using its ID,
then log in with the invited email address and password. This is demo code,
not a production authentication service.

## Prerequisites

- PostgreSQL and its `libpq` development libraries.
- A C/C++ toolchain and libclang development libraries for native dependencies.
- A SparkPost EU account, API key, verified sending address, and network access
  to send invitation emails (`Transmission::new_eu` is used).
- The Diesel CLI with PostgreSQL support:

```sh
cargo install diesel_cli --no-default-features --features postgres
```

On Debian/Ubuntu, the native development packages include `libclang-dev` and
`libpq-dev`. On macOS with Homebrew's `libpq`, prefix Cargo commands with
`PQ_LIB_DIR=/opt/homebrew/opt/libpq/lib` if it is not found automatically.

## Run the example

```sh
cd extras/simple-auth-server
```

Create a local `.env` file with your own database credentials and SparkPost
settings; `dotenv::dotenv` loads it at startup:

```text
DATABASE_URL=postgres://username:password@localhost/ntex_auth
SPARKPOST_API_KEY=your-sparkpost-eu-api-key
SENDING_EMAIL_ADDRESS=you@your-verified-domain.example
SECRET_KEY=replace-with-at-least-32-random-bytes
DOMAIN=localhost
```

`DATABASE_URL` is required at startup. `SPARKPOST_API_KEY` and
`SENDING_EMAIL_ADDRESS` are required when sending invitations. `DOMAIN`
defaults to `localhost`; `SECRET_KEY` has an insecure built-in fallback, so
replace the placeholder with a random secret of at least 32 bytes. The secret
is used for both cookies and password hashing; keep it stable between runs.
Do not commit credentials.

From the crate directory, create the database and apply the checked-in
`migrations/` (including the `users` and `invitations` tables), then run:

```sh
diesel database setup
cargo run
```

Migrations are not run by the application. The database account needs the
appropriate permissions to create the database and tables.

The server listens on `127.0.0.1:3000`. Cookies are named `auth`, last one day,
and use `secure(false)` for local HTTP.

## Try it

| Request | JSON body | Result |
| --- | --- | --- |
| `POST /api/invitation` | `{"email":"you@example.com"}` | Stores an invitation and sends email; empty success response |
| `POST /api/register/{invitation_id}` | `{"password":"your-password"}` | Registers the invited email and returns it as JSON |
| `POST /api/auth` | `{"email":"you@example.com","password":"your-password"}` | Sets the identity cookie; empty success response |
| `GET /api/auth` | None | Returns the logged-in email as JSON, or `401 Unauthorized` |
| `DELETE /api/auth` | None | Clears the identity cookie |

Replace the addresses, password, and invitation ID before sending requests:

```sh
curl -i -X POST http://localhost:3000/api/invitation \
  -H 'Content-Type: application/json' -d '{"email":"you@example.com"}'
curl -i -X POST http://localhost:3000/api/register/INVITATION_ID \
  -H 'Content-Type: application/json' -d '{"password":"your-password"}'
curl -i -c cookies.txt -X POST http://localhost:3000/api/auth \
  -H 'Content-Type: application/json' \
  -d '{"email":"you@example.com","password":"your-password"}'
curl -b cookies.txt http://localhost:3000/api/auth
curl -i -b cookies.txt -c cookies.txt -X DELETE http://localhost:3000/api/auth
```

Use the invitation ID from the email link with the registration API directly.
The emailed link points to `/register.html`, but `src/main.rs` does not serve
`static/` or register that page, so the browser registration link currently
returns a not-found response. Registration takes the email from the invitation,
not from the request body. Remove `cookies.txt` when finished.

## Tests

```sh
cargo test -p simple-auth-server
```

Checks unauthenticated requests, invalid invitation IDs, and logout without connecting to PostgreSQL.

## Learn more

- [Scopes](https://docs.ntex.rs/8.1-web-app.html#scopes)
- [Application State](https://docs.ntex.rs/8.2-web-app.html#accessing-application-state-in-handlers)
