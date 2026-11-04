# Docker HTTP server

A small ntex application packaged with a multi-stage Dockerfile. In
`src/main.rs`, `web::server` registers two `#[web::get]` handlers that return
plain-text greetings.

## Prerequisites

Docker is needed only for the container commands below.

## Run the example

```sh
cd server/docker
cargo run
```

The server listens on `0.0.0.0:5000`.

## Try it

| Request | Result |
| --- | --- |
| `GET /` | `Hello world!` |
| `GET /again` | `Hello world again!` |

```sh
curl http://localhost:5000/
curl http://localhost:5000/again
```

## Build and run a container

Run this build command from the **repository root**, not `server/docker`.
The build context must include the workspace `Cargo.toml`, because this crate
inherits its `ntex` dependency from the workspace.

```sh
docker build -f server/docker/Dockerfile -t docker_sample .
```

The supplied Dockerfile needs updating for the workspace layout: its
`cargo build` has no `-p docker_sample`, so it attempts to build the entire
workspace for `x86_64-unknown-linux-musl`, not just this example. The command
above supplies the correct context but does not fix that build limitation;
it is not a verified working container build.

After an image has been built successfully, run it with:

```sh
docker run -d -p 5000:5000 docker_sample
docker ps
```

The server listens on `0.0.0.0:5000` inside the container; the port mapping
makes it available at <http://localhost:5000/> on the host. `-d` already runs
the container in the background, so no shell `&` is needed.

## Tests

```sh
cargo test -p docker_sample
```

Covers both greeting routes and unknown routes without starting Docker or a listener.

## Learn more

- [Starting a server](https://docs.ntex.rs/5-server.html#starting-a-server) and [binding listeners](https://docs.ntex.rs/5-server.html#binding-and-existing-listeners)
- [Handlers](https://docs.ntex.rs/8.1-web-app.html#handlers) and [modular configuration](https://docs.ntex.rs/8.1-web-app.html#modular-configuration)
- [Testing routes](https://docs.ntex.rs/8.1-web-app.html#testing-routes)
