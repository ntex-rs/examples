# GraphQL with Juniper

Integrate [Juniper](https://github.com/graphql-rust/juniper) with ntex using a
shared `Schema` in application state. `src/main.rs` accepts JSON-encoded
`GraphQLRequest` values and executes them through `web::block`;
`src/schema.rs` defines the query, mutation, and GraphQL types.

No database is needed. The query returns fixed demonstration data, and the
mutation returns its input with a fixed ID; it does not persist anything.

## Run the example

```sh
cd db/juniper
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| `GET /graphiql` | Open the GraphiQL interface |
| `POST /graphql` | Execute a GraphQL request supplied as JSON |

Open <http://127.0.0.1:8080/graphiql> and try this query:

```text
{
  human(id: "1234") {
    id
    name
    appearsIn
    homePlanet
  }
}
```

The resolver uses the supplied ID and returns `Luke`, `[NEW_HOPE]`, and
`Mars` for the other fields. Try the mutation with an explicit episode list:

```text
mutation {
  createHuman(newHuman: {
    name: "Fresh Kid Ice"
    appearsIn: [EMPIRE]
    homePlanet: "earth"
  }) {
    id
    name
    appearsIn
    homePlanet
  }
}
```

It returns ID `1234` and the supplied name, episodes, and planet. Subsequent
queries still return the fixed query data.

The same API accepts JSON requests from curl:

```sh
curl http://127.0.0.1:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ human(id: \"1234\") { name appearsIn homePlanet } }"}'
```

## Tests

```sh
cargo test -p juniper-example
```

Exercises the GraphQL query, GraphiQL page, and invalid-field response through the in-process app.

## Learn more

- [Application state](https://docs.ntex.rs/8.2-web-app.html)
- [Extractors](https://docs.ntex.rs/8.4-web-app.html#extractors)
