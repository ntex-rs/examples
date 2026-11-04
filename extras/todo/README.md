# PostgreSQL todo application

This example renders a todo list with `Tera`, persists tasks with Diesel and
PostgreSQL, and uses `CookieSession` for flash messages after form submissions.
Look at `src/api.rs` for the handlers, `src/db.rs` and `src/model.rs` for
blocking database work through `web::block`, and `src/main.rs` for worker-local
template initialization with `ServerAppConfig`.

It is based on the [Rocket todo example](https://github.com/SergioBenitez/Rocket/tree/master/examples/todo),
using PostgreSQL rather than SQLite.

## Prerequisites

- Rust 1.97 or newer, matching the workspace's minimum supported version.
- PostgreSQL and its `libpq` development libraries.
- The Diesel CLI with PostgreSQL support:

```sh
cargo install diesel_cli --no-default-features --features postgres
```

On macOS with Homebrew's `libpq`, prefix Cargo commands with
`PQ_LIB_DIR=/opt/homebrew/opt/libpq/lib` if it is not found automatically.

## Run the example

```sh
cd extras/todo
```

Check the included `.env` file. Its default is
`DATABASE_URL=postgres://localhost/ntex_todo`. If authentication is required,
replace it with your own credentials, for example:

```text
DATABASE_URL=postgres://username:password@localhost/ntex_todo
```

Keep credentials local. From this directory, create the database and apply the
checked-in `migrations/`, then run:

```sh
diesel database setup
cargo run
```

Migrations create the `tasks` table and are not run automatically by the
application. The database account needs permission to create the database
and tables.

The server listens on `localhost:8088`. Run from the crate directory so that
`templates/` and `static/` resolve correctly. The fixed session signing key
and `secure(false)` cookies are for local demonstration only.

## Try it

Open <http://localhost:8088/> to add tasks, toggle completion, and delete them.
Flash messages are stored in the signed session cookie and shown after a
redirect.

| Request | Result |
| --- | --- |
| `GET /` | Renders the task list |
| `POST /todo` | Creates a task from the `description` form field |
| `POST /todo/{id}` | Toggles completion with `_method=put` or deletes with `_method=delete` |
| `GET /static/{path}` | Serves files from `static/` |

## Tests

```sh
cargo test -p todo
```

Checks empty-description redirects and unsupported update methods without connecting to PostgreSQL.

## Learn more

- [Request bodies](https://docs.ntex.rs/8.4-web-app.html#request-bodies)
- [Worker and Request State](https://docs.ntex.rs/6-server-app.html#creating-the-worker-application)
