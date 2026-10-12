//! A minimal middleware that shows the life cycle of an ntex middleware.
//!
//! A middleware is made of two parts:
//!
//! 1. A *factory* (`SayHi`), a type implementing [`Middleware`]. It holds the
//!    middleware configuration and is passed to `App::middleware()`.
//! 2. A *service* (`SayHiMiddleware`), a type implementing [`Service`]. The
//!    factory creates it, and it handles every request.
//!
//! # Initialization
//!
//! The server builds the application once for each worker thread. While
//! building it, ntex first creates the inner part of the pipeline (routing,
//! resources and handlers), and then calls [`Middleware::create()`] on each
//! middleware factory, passing the service built so far. The factory returns a
//! new service that *wraps* the one it was given, and that service becomes
//! the next one in the chain for the middleware added before it.
//!
//! With `App::new().middleware(A).middleware(B)` the chain looks like this:
//!
//! ```text
//! request  -> A -> B -> router -> handler
//! response <- A <- B <- router <- handler
//! ```
//!
//! Requests pass through middleware in the order it was added, responses
//! travel back in the opposite order.
//!
//! # Processing requests
//!
//! For every request ntex calls [`Service::call()`] on the outermost service.
//! The middleware can inspect or modify the request, then pass it to the
//! service it wraps and await the response, and finally inspect or modify the
//! response before returning it. It can also skip the wrapped service and
//! return a response of its own, see `redirect.rs`.
use ntex::http::header::{HeaderName, HeaderValue};
use ntex::service::{Ctx, Middleware, Service};
use ntex::web::{State, WebRequest, WebResponse};

/// Middleware factory.
///
/// This type is passed to `App::middleware()`. It can carry configuration
/// (for example a header name or a timeout), which is then copied into each
/// service it creates. `SayHi` has no configuration, so it is an empty struct.
pub struct SayHi;

/// Implementing `Middleware` turns `SayHi` into a middleware factory.
///
/// * `S` - type of the next service in the chain, the service being wrapped.
///   The concrete type is chosen by ntex when the application is built, it is
///   the router or the service created by the next middleware.
/// * `St` - type of the application state, see `web::App::state()`.
impl<S, St> Middleware<S, St> for SayHi {
    /// Service created by this factory.
    type Service = SayHiMiddleware<S>;

    /// Called once per worker thread when the application is built, not for
    /// each request.
    ///
    /// `service` is the next service in the chain. It is passed by value, so
    /// the new middleware service takes ownership of it and becomes the only
    /// way to reach it: every request to the wrapped service has to go through
    /// `SayHiMiddleware::call()`.
    fn create(&self, _state: &St, service: S) -> Self::Service {
        // Store the wrapped service inside the middleware service. Any
        // per-worker data the middleware needs (counters, caches, cloned
        // configuration) can be initialized here as well.
        SayHiMiddleware { service }
    }
}

/// Middleware service, it handles requests.
///
/// One instance exists for each worker thread. It is shared by all the
/// requests that the worker processes, so `call()` takes `&self`. State that
/// changes between requests needs interior mutability, e.g. `Cell` or `RefCell`.
pub struct SayHiMiddleware<S> {
    /// The next service in the chain, owned by this middleware.
    service: S,
}

/// Implementing `Service` makes the middleware a request handler.
///
/// The wrapped service has to accept `WebRequest` and produce `WebResponse`,
/// so `SayHiMiddleware` can forward the request to it and return its response.
impl<S, St: State> Service<St, WebRequest> for SayHiMiddleware<S>
where
    S: Service<St, WebRequest, Res = WebResponse>,
{
    /// The middleware returns the same response type as the wrapped service.
    type Res = WebResponse;
    /// It does not produce errors of its own, so it reuses the error type of
    /// the wrapped service and passes its errors through unchanged.
    type Error = S::Error;

    // `ready()` is called before `call()` to check that the service can accept
    // a request; it lets a service apply backpressure. This middleware is
    // always ready itself, so readiness is the readiness of the wrapped
    // service.
    ntex::forward_ready!(St, service);
    // `shutdown()` is called when the server stops. The middleware owns the
    // wrapped service, so it has to forward the shutdown signal to it.
    ntex::forward_shutdown!(St, service);

    /// Called for every request that reaches this middleware.
    ///
    /// `ctx` is the call context, use it to call the wrapped service. It takes
    /// care of readiness and of the service life cycle.
    async fn call(
        &self,
        mut req: WebRequest,
        ctx: Ctx<'_, Self, St>,
    ) -> Result<Self::Res, Self::Error> {
        // 1. Before the request is passed on.
        //
        // The middleware owns the request at this point, so it can read and
        // change it: headers, extensions, the payload. Services further down
        // the chain, including the handler, see the changes.
        println!("Hi from start. You requested: {}", req.path());
        req.headers_mut().insert(
            HeaderName::from_static("x-say-hi"),
            HeaderValue::from_static("hi"),
        );

        // 2. Call the next service in the chain.
        //
        // Ownership of the request moves to the wrapped service. The `?`
        // returns errors of the wrapped service to the caller unchanged. A
        // middleware that does not call the wrapped service at all, e.g. to
        // reject a request, returns its own response instead.
        let mut res = ctx.call(&self.service, req).await?;

        // 3. After the response is produced.
        //
        // The middleware now owns the response and can change its status or
        // headers, or replace the body with `WebResponse::map_body()`, see
        // `read_response_body.rs`.
        println!("Hi from response");
        res.headers_mut().insert(
            HeaderName::from_static("x-said-hi"),
            HeaderValue::from_static("yes"),
        );

        // The response travels back to the middleware added before this one,
        // and from there to the client.
        Ok(res)
    }
}
