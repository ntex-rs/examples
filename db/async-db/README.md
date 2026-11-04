# Asynchronous SQLite queries

Compare sequential and concurrent database queries without blocking ntex workers.
`src/main.rs` uses `AppState` to share an `r2d2` connection pool, while
`src/db.rs` runs synchronous SQLite queries through `web::block`.

Each request runs four weather queries. An artificial two-second delay per
query makes the difference between sequential execution and `join_all` easier
to see; this is a demonstration, not a database benchmark.

## Prerequisites

Install the SQLite development library and the `sqlite3` command-line tool.
On Ubuntu, for example:

```sh
sudo apt install libsqlite3-dev sqlite3
```

The included data comes from NOAA; see [the database notes](db/README.md).

## Run the example

From the repository root, initialize the database once, then start the server:

```sh
cd db/async-db
bash db/setup_db.sh
cargo run
```

The server listens on `127.0.0.1:8080`. The setup script creates
`db/async-db/weather.db`, and the application opens `weather.db` relative to the
current working directory. Keep that directory at `db/async-db` when running.
Do not rerun the import on an existing database: it appends the CSV rows again.

## Try it

| Request | Result |
| --- | --- |
| `GET /asyncio_weather` | Run the four queries sequentially and return their results as JSON |
| `GET /parallel_weather` | Run the same queries concurrently and return their results as JSON |

```sh
curl http://127.0.0.1:8080/asyncio_weather
curl http://127.0.0.1:8080/parallel_weather
```

Both responses contain four arrays: hottest years, coldest years, hottest
months, and coldest months, in that order. The rankings use sums of `tmax`.

To inspect the database from `db/async-db`:

```sh
sqlite3 weather.db
```

```text
.tables
SELECT * FROM nyc_weather LIMIT 10;
```

## Tests

```sh
cargo test -p async_db
```

Exercises sequential and concurrent weather routes and database errors using in-memory SQLite, without the simulated query delay.

## Learn more

- [Application state](https://docs.ntex.rs/8.2-web-app.html)
