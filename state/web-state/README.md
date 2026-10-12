Application State with Middleware, Filters and Handlers
-------------------------------------------------------

See also the [Application state](https://ntex.rs/docs/application/#state),
[Shared mutable state](https://ntex.rs/docs/application/#shared-mutable-state),
[Middleware](https://ntex.rs/docs/middleware/),
[Request handlers](https://ntex.rs/docs/handlers/) and
[Errors](https://ntex.rs/docs/errors/) sections of the ntex guide.

An ntex application has a single state type that implements the `web::State`
trait. The server calls the state factory once in every worker thread, so each
worker owns its own state value. The state does not have to be `Send` or
`Sync`, `Rc<Cell<T>>` works fine. Data that all workers share is created
outside of the state factory and cloned in, for example as an `Arc`.

Everything that processes requests gets a reference to the state:

| Where | How it gets the state |
|-------|-----------------------|
| Middleware | `Middleware::create(&self, st, service)` when the app is built, `ctx.st()` for every request |
| Filters | First argument of the filter function, wrapped with `fn_service_st()` |
| Handlers | First argument of handlers registered with `to_with_state()` |
| Errors | `WebResponseError::error_response(&self, st)` |

Every request also carries a *request state*. It starts as `()`. A filter can
replace it with another type using `WebRequest::map_state()`, and handlers
registered with `to_with_state()` receive it as the second argument.

Enforcing handler requirements at compile time
----------------------------------------------

The request state is part of the request type, `WebRequest<S>`. In this example
the filters change it twice:

```text
WebRequest<()>                  initial request
  -> authenticate               app filter
WebRequest<Option<User>>        maybe authenticated
  -> require_admin              /admin scope filter, 401/403 on failure
WebRequest<User>                authenticated administrator
```

A handler states what it needs with the type of its second argument, and a
route accepts only handlers that match the request state where the route is
registered:

```rust
// Anonymous users are allowed, so this is registered on the app.
async fn index(st: &AppState, user: Option<User>) -> String { ... }

// Requires an administrator, so it only fits in the /admin scope.
async fn admin_users(st: &AppState, admin: User) -> String { ... }
```

Only `require_admin` creates a `WebRequest<User>`, so a handler that takes a
`User` can't run unless that check succeeded. Registering `admin_users` on the
app instead of the `/admin` scope is a compile error:

```text
error[E0308]: mismatched types
  expected `Route<AppState, Option<User>>`, found `Route<AppState, User>`
```

The authorization check can't be forgotten when a route is added, and handlers
don't have to repeat it.

What it shows
-------------

* `AppState` holds the application name, a user map shared by all workers
  (`Arc`) and a request counter owned by each worker (`Rc<Cell<u64>>`).
* `RequestCounter` middleware uses the state in `create()` to prepare the
  `x-served-by` header, and `ctx.st()` to count requests and add the
  `x-request-count` header.
* The `authenticate` app filter looks up the `Authorization: Bearer <token>`
  user in the state and turns `WebRequest<()>` into `WebRequest<Option<User>>`.
* The `require_admin` filter on the `/admin` scope turns
  `WebRequest<Option<User>>` into `WebRequest<User>`, or stops the request with
  `AuthError`. `AuthError` uses the state to name the realm in
  `WWW-Authenticate`.
* Handlers use `to_with_state()` to get both states, and can still use
  extractors such as `Path`.

Running the example
-------------------

```bash
cargo run -p web-state
```

The server listens on `http://127.0.0.1:8080`. Two users are configured:

| Token | User | Admin |
|-------|------|-------|
| `alice-token` | alice | yes |
| `bob-token` | bob | no |

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Greets the user, or anonymous |
| GET | `/stats` | Requests served by this worker |
| GET | `/users/{name}` | User info |
| GET | `/admin/users` | Lists users, administrators only |

```bash
# Anonymous and authenticated requests
curl -i http://127.0.0.1:8080/
curl -i -H "Authorization: Bearer bob-token" http://127.0.0.1:8080/
curl -i -H "Authorization: Bearer alice-token" http://127.0.0.1:8080/users/alice

# The counter is per worker. Requests on one connection go to the same worker.
curl http://127.0.0.1:8080/stats http://127.0.0.1:8080/stats http://127.0.0.1:8080/stats

# 401, 403 and 200 from the /admin scope filter
curl -i http://127.0.0.1:8080/admin/users
curl -i -H "Authorization: Bearer bob-token" http://127.0.0.1:8080/admin/users
curl -i -H "Authorization: Bearer alice-token" http://127.0.0.1:8080/admin/users
```

Error responses from filters reach the middleware as errors, so they do not
get the `x-served-by` and `x-request-count` headers.
