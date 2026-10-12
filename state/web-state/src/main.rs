//! Application state with middleware, filters and handlers.
//!
//! An ntex application has one state type, `AppState` here. The server
//! creates a state value for every worker thread and passes a reference to it
//! to everything that processes requests:
//!
//! * middleware gets it when it is created (`Middleware::create()`) and for
//!   every request (`ctx.st()`),
//! * filters get it as the first argument,
//! * handlers registered with `to_with_state()` get it as the first argument,
//! * errors get it when they are rendered (`WebResponseError::error_response()`).
//!
//! Besides the application state, every request carries a *request state*.
//! It starts as `()`, and filters can replace it with a value of another type,
//! for example the authenticated user. Handlers registered with
//! `to_with_state()` receive it as the second argument.
//!
//! # Request state types enforce handler requirements at compile time
//!
//! The request state is part of the request type, `WebRequest<S>`. In this
//! example it changes twice:
//!
//! ```text
//! WebRequest<()>                       initial request
//!   -> authenticate (app filter)
//! WebRequest<Option<User>>             maybe authenticated
//!   -> require_admin (/admin scope filter)
//! WebRequest<User>                     authenticated administrator
//! ```
//!
//! Every handler declares the request state it needs as its second argument,
//! and a route only accepts handlers whose type matches the request state at
//! the place where the route is registered:
//!
//! * `index(st, user: Option<User>)` works for anonymous and authenticated
//!   users, so it is registered on the app, after `authenticate`.
//! * `admin_users(st, admin: User)` needs an administrator. A `User` value
//!   exists only after `require_admin` succeeded, so this handler can only be
//!   registered inside the `/admin` scope.
//!
//! Registering `admin_users` on the app instead of the scope does not
//! compile:
//!
//! ```text
//! error[E0308]: mismatched types
//!   expected `Route<AppState, Option<User>>`, found `Route<AppState, User>`
//! ```
//!
//! The access check can't be forgotten, and the handler doesn't have to repeat
//! it. If a handler requires an authenticated user, the compiler guarantees
//! that the filter that produces the user runs before it.
//!
//! See the guide: <https://ntex.rs/docs/application#state>
use std::{cell::Cell, collections::HashMap, convert::Infallible, io, rc::Rc, sync::Arc};

use ntex::http::header::{self, HeaderName, HeaderValue};
use ntex::service::{Middleware, Service, fn_service_st};
use ntex::web::{self, App, DefaultError, HttpResponse, WebRequest, WebResponse};
use ntex::{Ctx, SharedCfg};

/// An authenticated user.
#[derive(Clone, Debug)]
struct User {
    name: &'static str,
    admin: bool,
}

/// Application state.
///
/// ntex calls the state factory once in every worker thread, and every
/// worker stores its own value. The state must be `Clone`, but it does not
/// have to be `Send` or `Sync`, because it never leaves its worker.
#[derive(Clone)]
struct AppState {
    /// Application name, used in headers and error responses.
    name: &'static str,
    /// Users by access token.
    ///
    /// The map is created once, outside of the state factory, and every
    /// worker gets a clone of the `Arc`. All workers share the same data.
    users: Arc<HashMap<&'static str, User>>,
    /// Number of requests served by this worker.
    ///
    /// Created inside the state factory, so every worker has its own
    /// counter. `Rc<Cell<_>>` is enough, no locking is needed.
    requests: Rc<Cell<u64>>,
}

/// Enables `AppState` to be used as `web::App` state.
///
/// `Error` selects the application's error domain. `DefaultError` is the one
/// supported by all built-in ntex errors and extractors.
impl web::State for AppState {
    type Error = DefaultError;
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

/// Counts requests and adds the `x-served-by` and `x-request-count` headers.
///
/// Middleware is a factory. ntex calls `create()` once per worker, while it
/// builds the application, with the worker's state and the next service in
/// the chain. See the guide: <https://ntex.rs/docs/middleware>
struct RequestCounter;

impl<S> Middleware<S, AppState> for RequestCounter {
    type Service = RequestCounterService<S>;

    fn create(&self, st: &AppState, service: S) -> Self::Service {
        // The state is available while the application is being built, so
        // values that depend on it can be prepared once instead of for
        // every request.
        log::info!("Creating request counter middleware for {:?}", st.name);

        RequestCounterService {
            service,
            served_by: HeaderValue::from_static(st.name),
        }
    }
}

/// The service created by `RequestCounter`. It wraps the next service.
struct RequestCounterService<S> {
    service: S,
    served_by: HeaderValue,
}

/// The service's state type is `AppState`, so `ctx.st()` returns `&AppState`.
///
/// Application middleware runs before the filters, so it receives the
/// initial request state. It stays generic over it (`ReqSt`) and does not
/// depend on what the filters do.
impl<S, ReqSt> Service<AppState, WebRequest<ReqSt>> for RequestCounterService<S>
where
    S: Service<AppState, WebRequest<ReqSt>, Res = WebResponse>,
{
    type Res = WebResponse;
    type Error = S::Error;

    ntex::forward_ready!(AppState, service);
    ntex::forward_shutdown!(AppState, service);

    async fn call(
        &self,
        req: WebRequest<ReqSt>,
        ctx: Ctx<'_, Self, AppState>,
    ) -> Result<Self::Res, Self::Error> {
        // Access the worker's state for this request.
        let st = ctx.st();
        let count = st.requests.get() + 1;
        st.requests.set(count);

        // Call the filters and the router. An error returned by a filter or
        // a handler reaches the middleware as `Err`, so `?` passes it on and
        // the headers below are added only to successful responses.
        let mut res = ctx.call(&self.service, req).await?;

        let headers = res.headers_mut();
        headers.insert(
            HeaderName::from_static("x-served-by"),
            self.served_by.clone(),
        );
        headers.insert(
            HeaderName::from_static("x-request-count"),
            HeaderValue::from(count),
        );
        Ok(res)
    }
}

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

/// Application filter: finds the user for the `Authorization: Bearer` token.
///
/// Filters run after the application middleware and before routing. A filter
/// written as an async function gets the application state as the first
/// argument (see `fn_service_st()` in `main`) and the request as the second.
///
/// The filter changes the request type from `WebRequest<()>` to
/// `WebRequest<Option<User>>` with `map_state()`. From here on, every app
/// route must use handlers that accept `Option<User>`. Anonymous requests are
/// allowed, so the filter never fails.
async fn authenticate(
    st: &AppState,
    req: WebRequest<()>,
) -> Result<WebRequest<Option<User>>, Infallible> {
    let user = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| st.users.get(token))
        .cloned();

    if let Some(user) = &user {
        log::info!("Authenticated {:?}", user.name);
    }

    Ok(req.map_state(|()| user))
}

/// Errors returned by `require_admin`.
#[derive(Debug, thiserror::Error)]
enum AuthError {
    #[error("authentication required")]
    Unauthorized,
    #[error("{0} is not an administrator")]
    Forbidden(&'static str),
}

/// Renders `AuthError` responses.
///
/// `error_response()` receives the application state. Here it is used to
/// name the protection realm in the `WWW-Authenticate` header.
/// See the guide: <https://ntex.rs/docs/errors>
impl web::WebResponseError<AppState, DefaultError> for AuthError {
    fn error_response(&self, st: &AppState) -> HttpResponse {
        match self {
            AuthError::Unauthorized => HttpResponse::Unauthorized()
                .header(
                    header::WWW_AUTHENTICATE,
                    format!("Bearer realm=\"{}\"", st.name),
                )
                .body(format!("{self}\n")),
            AuthError::Forbidden(_) => HttpResponse::Forbidden().body(format!("{self}\n")),
        }
    }
}

/// Scope filter: allows only administrators into the `/admin` scope.
///
/// It runs after `authenticate`, so it receives `WebRequest<Option<User>>`
/// and turns it into `WebRequest<User>`. This is the only place in the
/// application where a `WebRequest<User>` is created. As a result, handlers
/// that take a `User` can only be registered behind this filter, and the
/// compiler rejects them anywhere else. They get a checked administrator and
/// don't have to check again.
///
/// Returning an error stops the request. ntex does not call the handler and
/// renders the error with `AuthError::error_response()`.
async fn require_admin(
    _: &AppState,
    req: WebRequest<Option<User>>,
) -> Result<WebRequest<User>, AuthError> {
    match req.st() {
        None => Err(AuthError::Unauthorized),
        Some(user) if !user.admin => Err(AuthError::Forbidden(user.name)),
        // The user is checked above, change the request state type.
        Some(_) => Ok(req.map_state(|user| user.unwrap())),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------
//
// Handlers registered with `to_with_state()` receive the application state
// and the request state as their first two arguments, followed by any number
// of extractors. See the guide: <https://ntex.rs/docs/handlers>
//
// The type of the second argument is the handler's requirement. It must match
// the request state where the route is registered:
//
//   Option<User>  app routes, the user may be anonymous
//   User          /admin scope routes, the user is an administrator

/// `GET /`: greets the user found by the `authenticate` filter.
///
/// It takes `Option<User>`, so it must handle anonymous requests.
async fn index(st: &AppState, user: Option<User>) -> String {
    match user {
        Some(user) => format!("Hello {}, welcome to {}!\n", user.name, st.name),
        None => format!("Hello anonymous, welcome to {}!\n", st.name),
    }
}

/// `GET /stats`: shows the worker's request counter.
///
/// The request state is not needed here, so it is ignored with `_`.
async fn stats(st: &AppState, _: Option<User>) -> String {
    format!("requests served by this worker: {}\n", st.requests.get())
}

/// `GET /users/{name}`: combines both states with the `Path` extractor.
async fn user_info(
    st: &AppState,
    user: Option<User>,
    name: web::types::Path<String>,
) -> HttpResponse {
    let Some(found) = st.users.values().find(|u| u.name == name.as_str()) else {
        return HttpResponse::NotFound().body(format!("user {name} not found\n"));
    };

    let role = if found.admin { "admin" } else { "user" };
    if user.is_some_and(|user| user.name == found.name) {
        HttpResponse::Ok().body(format!("{} ({role}), this is you\n", found.name))
    } else {
        HttpResponse::Ok().body(format!("{} ({role})\n", found.name))
    }
}

/// `GET /admin/users`: lists all users.
///
/// It takes `User`, so it requires an administrator. Only the `/admin` scope
/// filter produces a `WebRequest<User>`, so this handler can only run after
/// that check succeeded. Registering it next to `index`, where the request
/// state is `Option<User>`, is a compile error.
async fn admin_users(st: &AppState, admin: User) -> String {
    let mut names: Vec<_> = st.users.values().map(|u| u.name).collect();
    names.sort_unstable();
    format!(
        "{} users, requested by {}: {}\n",
        names.len(),
        admin.name,
        names.join(", ")
    )
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("ntex=info,web_state=info"));

    // Shared, read-only data. It is created once, outside of the state
    // factory, so all workers use the same map.
    let users = Arc::new(HashMap::from([
        (
            "alice-token",
            User {
                name: "alice",
                admin: true,
            },
        ),
        (
            "bob-token",
            User {
                name: "bob",
                admin: false,
            },
        ),
    ]));

    web::server_with_config(
        // State factory. It is called once in every worker thread.
        async move || {
            Ok(AppState {
                name: "web-state",
                users: users.clone(),
                // Created here, so every worker gets its own counter.
                requests: Rc::new(Cell::new(0)),
            })
        },
        // Application factory. It is called once in every worker thread,
        // after the state factory, with a reference to the worker's state.
        async |_: &AppState| {
            App::new()
                // Middleware runs first and gets `WebRequest<()>`.
                // Logger is registered first, so it wraps everything,
                // including `RequestCounter`.
                .middleware(web::middleware::Logger::default())
                .middleware(RequestCounter)
                // Filters run after the middleware and before routing.
                // `fn_service_st()` passes the application state to the
                // filter function.
                //
                // WebRequest<()> -> WebRequest<Option<User>>
                .filter(fn_service_st(authenticate))
                // App routes now require handlers that accept `Option<User>`.
                .route("/", web::get().to_with_state(index))
                .route("/stats", web::get().to_with_state(stats))
                .route("/users/{name}", web::get().to_with_state(user_info))
                .service(
                    web::scope("/admin")
                        // Scope filters run only for requests that match the
                        // scope.
                        //
                        // WebRequest<Option<User>> -> WebRequest<User>
                        .filter(fn_service_st(require_admin))
                        // Scope routes require handlers that accept `User`.
                        .route("/users", web::get().to_with_state(admin_users)),
                )
        },
    )
    .bind("127.0.0.1:8080", SharedCfg::new("WEB-STATE"))?
    .run()
    .await
}
