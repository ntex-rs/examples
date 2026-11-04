# Asynchronous PostgreSQL

Insert users with `tokio_postgres`, map rows with `tokio_pg_mapper`, and share
a `deadpool_postgres` connection pool through ntex application state.
`types::Json<User>` extracts the request body; `dotenv` and `config` load the
server and database settings in `src/main.rs`.

## Prerequisites

You need a running PostgreSQL instance and its `createuser`, `createdb`, and
`psql` tools. Create a role and database, or substitute your existing ones:

```sh
createuser -P test_user
createdb -O test_user testing_db
```

Enter `testing` as the password for this local example. From the repository
root, initialize the schema as that role:

```sh
psql -h 127.0.0.1 -U test_user -d testing_db -f db/async-pg/sql/schema.sql
```

This script drops and recreates the `testing` schema, deleting its existing
data. Create `db/async-pg/.env` with these settings, adjusting credentials and
addresses for your installation:

```text
SERVER_ADDR=127.0.0.1:8080
PG.USER=test_user
PG.PASSWORD=testing
PG.HOST=127.0.0.1
PG.PORT=5432
PG.DBNAME=testing_db
PG.POOL.MAX_SIZE=16
```

The dependency is named `dotenv`; it reads this file when the application
starts from the crate directory. Database connections use `NoTls`, so this
configuration is intended for a local PostgreSQL instance.

## Run the example

From the repository root:

```sh
cd db/async-pg
cargo run
```

With the configuration above, the server listens on `127.0.0.1:8080`.
Change `SERVER_ADDR` to use another address.

## Try it

```sh
curl -i http://127.0.0.1:8080/users \
  -H "Content-Type: application/json" \
  -d '{"email":"ferris@thecrab.com","first_name":"ferris","last_name":"crab","username":"ferreal"}'
```

`POST /users` requires all four string fields and returns the inserted user
as JSON. The `username` column is unique: repeating this request returns
HTTP 500. There is no route for listing users.

## Tests

```sh
cargo test -p async_pg
```

Checks rendered not-found and connection-pool error responses without connecting to PostgreSQL.

## Learn more

- [Application state](https://docs.ntex.rs/8.2-web-app.html)
- [Extractors](https://docs.ntex.rs/8.4-web-app.html#extractors)
