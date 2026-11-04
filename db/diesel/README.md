# Diesel with SQLite

Use [Diesel](https://diesel.rs/) and an `r2d2` connection pool to create and
look up users in SQLite. `src/main.rs` shares the pool through `AppState`
and calls the synchronous queries in `src/actions.rs` through `web::block`
to keep database work off the server workers.

## Prerequisites

Install SQLite and its development library. For example:

```sh
# Ubuntu
sudo apt-get install libsqlite3-dev sqlite3

# Fedora
sudo dnf install sqlite-devel sqlite

# OpenSUSE
sudo zypper install sqlite3-devel libsqlite3-0 sqlite3

# macOS with Homebrew
brew install sqlite3
```

Install the SQLite-enabled Diesel CLI:

```sh
cargo install diesel_cli --no-default-features --features sqlite
```

The included `db/diesel/.env` sets `DATABASE_URL=test.db`. From the repository
root, apply the migrations before starting the application:

```sh
cd db/diesel
diesel migration run
```

This creates `test.db` and its `users` table. Migrations are not run by the
server. The database path is relative to the current working directory.

## Run the example

From the repository root:

```sh
cd db/diesel
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| `POST /user` | Insert a user from a JSON body containing `name`; return JSON with `id` and `name` |
| `GET /user/{user_id}` | Look up a UUID and return the user as JSON, or HTTP 404 if absent |

Create a user:

```sh
curl -i http://127.0.0.1:8080/user \
  -H "Content-Type: application/json" \
  -d '{"name":"bill"}'
```

Replace `USER_ID` below with the generated UUID from the response:

```sh
curl -i http://127.0.0.1:8080/user/USER_ID
```

To inspect the database from `db/diesel`:

```sh
sqlite3 test.db
```

```text
.tables
SELECT * FROM users;
```

## Tests

```sh
cargo test -p diesel-example
```

Exercises user creation, lookup, and invalid or missing IDs using migrated in-memory SQLite.

## Learn more

- [Application state](https://docs.ntex.rs/8.2-web-app.html)
- [Extractors](https://docs.ntex.rs/8.4-web-app.html#extractors)
- [Diesel with PostgreSQL in FrameworkBenchmarks](https://github.com/TechEmpower/FrameworkBenchmarks/tree/master/frameworks/Rust/ntex)
