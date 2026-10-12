//! A middleware that reads the response body produced by the handler.
//!
//! A response body is not necessarily in memory when the handler returns: it
//! can be a stream that produces chunks while the response is being sent. So
//! the middleware cannot simply read the body in `call()`. Instead it replaces
//! the body with a wrapper, `BodyLogger`, which passes every chunk on to the
//! client unchanged and keeps a copy of it. When the response has been sent
//! and the wrapper is dropped, it prints everything it has seen.
//!
//! ```text
//! handler body -> BodyLogger (copies chunk) -> client
//! ```
//!
//! See `simple.rs` for a step by step description of how a middleware is
//! created and how it wraps the next service in the chain.
use std::{rc::Rc, task::Context, task::Poll};

use ntex::http::body::{Body, BodySize, MessageBody, ResponseBody};
use ntex::service::{Ctx, Middleware, Service};
use ntex::util::{Bytes, BytesMut};
use ntex::web::{WebRequest, WebResponse};

/// Middleware factory, passed to `App::middleware()`.
pub struct Logging;

/// Implementing `Middleware` turns `Logging` into a middleware factory.
///
/// * `S` - type of the next service in the chain, the service being wrapped.
/// * `St` - type of the application state.
impl<S, St> Middleware<S, St> for Logging {
    /// Service created by this factory.
    type Service = LoggingMiddleware<S>;

    /// Called once per worker thread when the application is built. The
    /// middleware service takes ownership of the next service in the chain.
    fn create(&self, _state: &St, service: S) -> Self::Service {
        LoggingMiddleware { service }
    }
}

/// Middleware service, one instance per worker thread.
pub struct LoggingMiddleware<S> {
    /// The next service in the chain, owned by this middleware.
    service: S,
}

impl<S, St> Service<St, WebRequest> for LoggingMiddleware<S>
where
    S: Service<St, WebRequest, Res = WebResponse>,
{
    type Res = WebResponse;
    /// Wrapping the body cannot fail, so the middleware reuses the error type
    /// of the wrapped service.
    type Error = S::Error;

    // Readiness and shutdown are delegated to the wrapped service.
    ntex::forward_ready!(St, service);
    ntex::forward_shutdown!(St, service);

    /// Called for every request that reaches this middleware.
    async fn call(&self, req: WebRequest, ctx: Ctx<'_, Self, St>) -> Result<WebResponse, S::Error> {
        // The request is passed on unchanged. The work happens on the
        // response, after the wrapped service has produced it; an error is
        // returned as is and has no body to wrap.
        ctx.call(&self.service, req).await.map(|res| {
            // `map_body()` takes the body out of the response and stores what
            // the closure returns in its place. The closure also gets the
            // response head, so it could change the status or headers too.
            res.map_body(move |_head, body| {
                // `Body::from_message()` boxes any `MessageBody`
                // implementation into a `Body`, and `into()` turns it into the
                // `ResponseBody` that the response stores.
                Body::from_message(BodyLogger {
                    body,
                    body_accum: BytesMut::new(),
                })
                .into()
            })
        })
    }
}

/// Response body wrapper, it copies every chunk of the wrapped body.
///
/// It lives as long as the response is being sent, not just during `call()`,
/// so it can observe bodies that are produced over time.
pub struct BodyLogger {
    /// The original body produced by the handler.
    body: ResponseBody<Body>,
    /// Copy of the chunks sent so far.
    ///
    /// This keeps the whole body in memory. A real middleware should limit
    /// what it keeps, e.g. only the first few kilobytes.
    body_accum: BytesMut,
}

/// The wrapper is dropped once the response has been sent, or when the
/// connection goes away before that, so this runs once for every wrapped
/// response.
impl Drop for BodyLogger {
    fn drop(&mut self) {
        println!("response body: {:?}", self.body_accum);
    }
}

/// `MessageBody` is the trait ntex uses to send a response body. The server
/// uses `size()` to choose between a `Content-Length` header and chunked
/// encoding, and then polls `poll_next_chunk()` until it returns `None`.
impl MessageBody for BodyLogger {
    /// The wrapper does not change the body, so it reports the size of the
    /// original one. This keeps the `Content-Length` header correct.
    fn size(&self) -> BodySize {
        self.body.size()
    }

    /// Polls the wrapped body and copies each chunk before passing it on.
    fn poll_next_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Bytes, Rc<dyn std::error::Error>>>> {
        match self.body.poll_next_chunk(cx) {
            // A chunk is ready: keep a copy and send the original. `Bytes`
            // is reference counted, so passing it on does not copy it.
            Poll::Ready(Some(Ok(chunk))) => {
                self.body_accum.extend_from_slice(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            // The body failed, pass the error on, the response is aborted.
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            // The body is complete.
            Poll::Ready(None) => Poll::Ready(None),
            // No chunk yet. The wrapped body has registered `cx`'s waker, and
            // the server polls again when it is woken.
            Poll::Pending => Poll::Pending,
        }
    }
}
