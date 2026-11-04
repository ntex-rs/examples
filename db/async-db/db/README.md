# Weather database

This directory contains NYC Central Park weather data from
[NOAA](https://www.ncdc.noaa.gov/cdo-web/), the `nyc_weather` table definition
in `db.sql`, and `setup_db.sh` to import the CSV with `sqlite3`.
`GHCND_documentation.pdf` describes the source data.

## Prerequisites

Install the `sqlite3` command-line tool. The Rust example also needs the
SQLite development library; see [the example README](../README.md).

## Set up the database

From the repository root:

```sh
cd db/async-db
bash db/setup_db.sh
```

The script changes to its own directory and creates `../weather.db`, which is
`db/async-db/weather.db` from the repository root. Run the server with
`cargo run` from `db/async-db` so it opens that same file.

Run the import only once on a fresh database. Repeating it attempts to create
an existing table and appends the CSV rows again.

## Inspect the data

From `db/async-db`:

```sh
sqlite3 weather.db
```

```text
.tables
SELECT * FROM nyc_weather LIMIT 10;
```
