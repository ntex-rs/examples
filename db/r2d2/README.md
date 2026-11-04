# SQLite connection pooling with r2d2

Share an `r2d2::Pool<SqliteConnectionManager>` through ntex `AppState`.
The handler in `src/main.rs` extracts a name with `types::Path`, then uses
`web::block` to insert it into SQLite and read it back without blocking a
server worker.

## Prerequisites

Install the SQLite development library and the `sqlite3` command-line tool.
For example, on Ubuntu:

```sh
sudo apt install libsqlite3-dev sqlite3
```

The application opens `test.db` in the current working directory, but does
not create the `users` table. Initialize it once from the repository root:

```sh
cd db/r2d2
sqlite3 test.db 'CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL);'
```

## Run the example

From the repository root:

```sh
cd db/r2d2
cargo run
```

The server listens on `127.0.0.1:8080`. Run from `db/r2d2` so the application
opens the database you initialized.

## Try it

| Request | Result |
| --- | --- |
| `GET /{name}` | Insert the name with a new UUID, read it back, and return a JSON string |

```sh
curl http://127.0.0.1:8080/Ferris
```

Each request inserts another row, even when the name is unchanged. This is a
small demonstration: pool acquisition and insertion failures can panic.

## Tests

```sh
cargo test -p r2d2-example
```

Checks name insertion, JSON responses, and unmatched routes using an in-memory SQLite database.

## Learn more

- [Application state](https://docs.ntex.rs/8.2-web-app.html)
- [Path variables](https://docs.ntex.rs/8.1-web-app.html#accessing-path-variables)
