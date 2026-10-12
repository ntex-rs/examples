//! A middleware that guards the application, a rough sketch of a login check.
//!
//! Unlike `simple.rs`, this middleware does not always call the service it
//! wraps. If the user is not logged in, it answers the request itself with a
//! redirect to `/login`. The rest of the chain (the remaining middleware, the
//! router and the handler) never sees that request.
//!
//! ```text
//! logged in:      request -> CheckLogin -> ... -> handler
//! not logged in:  request -> CheckLogin -> 302 Found, Location: /login
//! ```
//!
//! See `simple.rs` for a step by step description of how a middleware is
//! created and how it wraps the next service in the chain.
use ntex::http;
use ntex::service::{Ctx, Middleware, Service};
use ntex::web::{HttpResponse, WebRequest, WebResponse};

/// Middleware factory, passed to `App::middleware()`.
///
/// A real login check would carry its configuration here, e.g. the path of
/// the login page or the name of the session cookie.
pub struct CheckLogin;

/// Implementing `Middleware` turns `CheckLogin` into a middleware factory.
///
/// * `S` - type of the next service in the chain, the service being wrapped.
/// * `St` - type of the application state.
impl<S, St> Middleware<S, St> for CheckLogin {
    /// Service created by this factory.
    type Service = CheckLoginMiddleware<S>;

    /// Called once per worker thread when the application is built.
    ///
    /// The next service in the chain is passed by value, so the middleware
    /// service takes ownership of it. Requests can reach it only through
    /// `CheckLoginMiddleware::call()`, which is what makes the guard work: the
    /// middleware decides whether a request is passed on at all.
    fn create(&self, _state: &St, service: S) -> Self::Service {
        CheckLoginMiddleware { service }
    }
}

/// Middleware service, one instance per worker thread.
pub struct CheckLoginMiddleware<S> {
    /// The next service in the chain, owned by this middleware.
    service: S,
}

/// The wrapped service has to accept `WebRequest` and produce `WebResponse`.
///
/// The middleware returns `WebResponse` too, either the response of the
/// wrapped service or a redirect it builds itself. Both paths must produce
/// the same type, that is why the wrapped service is required to return
/// `WebResponse` rather than an arbitrary type.
impl<S, St> Service<St, WebRequest> for CheckLoginMiddleware<S>
where
    S: Service<St, WebRequest, Res = WebResponse>,
{
    type Res = WebResponse;
    /// Building the redirect cannot fail, so the middleware has no errors of
    /// its own and reuses the error type of the wrapped service.
    type Error = S::Error;

    // Readiness and shutdown are delegated to the wrapped service, the
    // middleware itself is always ready and holds no resources to release.
    ntex::forward_ready!(St, service);
    ntex::forward_shutdown!(St, service);

    /// Called for every request that reaches this middleware.
    async fn call(
        &self,
        req: WebRequest,
        ctx: Ctx<'_, Self, St>,
    ) -> Result<Self::Res, Self::Error> {
        // The whole decision is made before the request is passed on, the
        // response of the wrapped service is returned unchanged.

        // A real implementation would inspect the request here, e.g. read a
        // session cookie with `req.cookie("session")` or check an
        // `Authorization` header.
        let is_logged_in = false; // Change this to see the change in outcome in the browser

        if is_logged_in {
            // Authorized: pass the request on to the next service in the chain
            // and return whatever it produces.
            ctx.call(&self.service, req).await
        } else {
            // Don't forward to /login if we are already on /login, the login
            // page itself has to stay reachable or every request would be
            // redirected forever.
            if req.path() == "/login" {
                ctx.call(&self.service, req).await
            } else {
                // Short-circuit: answer the request here, without calling the
                // wrapped service.
                //
                // `into_response()` consumes the request and attaches the new
                // response to it, so the middleware added before this one
                // still gets a complete `WebResponse` with access to the
                // original request.
                Ok(req.into_response(
                    HttpResponse::Found()
                        .header(http::header::LOCATION, "/login")
                        .build()
                        .into_body(),
                ))
            }
        }
    }
}
