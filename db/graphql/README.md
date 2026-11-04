# GraphQL with MySQL

Combine ntex, [Juniper](https://github.com/graphql-rust/juniper), and MySQL
behind a GraphQL API. `src/handlers.rs` extracts a `GraphQLRequest` with
`types::Json`, then executes the synchronous schema through `web::block`.
Application state shares the schema and an `r2d2` pool.

The query and mutation definitions are in `src/schemas/root.rs`; the user
and product types live alongside them.

## Prerequisites

You need MySQL 8 and its `mysql` command-line client. From the repository root,
create a database and import the supplied schema, using an account with the
necessary permissions:

```sh
mysql -u root -p -e "CREATE DATABASE rust_graphql"
mysql -u root -p rust_graphql < db/graphql/mysql-schema.sql
```

The import drops and recreates the `user` and `product` tables. Create
`db/graphql/.env` with a connection URL, replacing `USER` and `PASSWORD` with
your MySQL credentials:

```text
DATABASE_URL=mysql://USER:PASSWORD@127.0.0.1:3306/rust_graphql
```

The checked-in `.env.example` contains a redacted URL; do not copy it unchanged.
The application uses `dotenv` to load `.env` from the crate directory.

## Run the example

From the repository root:

```sh
cd db/graphql
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| `GET /graphiql` | Open the GraphiQL interface for the API |
| `POST /graphql` | Execute a GraphQL request supplied as JSON |

Open <http://127.0.0.1:8080/graphiql>, or query users with curl:

```sh
curl http://127.0.0.1:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ users { id name email } }"}'
```

Create a user in GraphiQL:

```text
mutation {
  createUser(user: { name: "Ferris", email: "ferris@example.com" }) {
    id
    name
    email
  }
}
```

The schema also provides `user(id:)`, `products`, `product(id:)`, and
`createProduct(product:)`. Product input includes `userId`, `name`, and `price`;
use an existing user ID to satisfy the foreign key. User email addresses must
be unique.

## Tests

```sh
cargo test -p ntex-graphql-demo
```

Checks the GraphiQL page and malformed JSON rejection without connecting to MySQL.

## Learn more

- [Application state](https://docs.ntex.rs/8.2-web-app.html)
- [Extractors](https://docs.ntex.rs/8.4-web-app.html#extractors)
