//! A middleware that reads the request body before the handler runs.
//!
//! The request payload is a stream: its chunks arrive from the network while
//! they are read, and it can be consumed only once. To look at the body, the
//! middleware takes the stream out of the request, reads it to the end, and
//! then puts the collected bytes back, so the handler still gets the complete
//! body.
//!
//! Reading the payload can fail, so unlike `simple.rs` this middleware has
//! errors of its own and converts all errors into `WebError`.
//!
//! See `simple.rs` for a step by step description of how a middleware is
//! created and how it wraps the next service in the chain.
use futures::stream::{self, StreamExt};
use ntex::http::{Payload, PayloadStream, error};
use ntex::service::{Ctx, Middleware, Service};
use ntex::util::BytesMut;
use ntex::web::{State, WebError, WebRequest, WebResponse, WebResponseError};

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

/// The middleware can fail in two ways: reading the payload returns a
/// `PayloadError`, and the wrapped service returns `S::Error`. Both are
/// converted into `WebError`, the error type used by ntex web services, so
/// both must implement `WebResponseError`, which tells ntex how to turn the
/// error into an HTTP response.
impl<S, St> Service<St, WebRequest> for LoggingMiddleware<S>
where
    St: State,
    S: Service<St, WebRequest, Res = WebResponse> + 'static,
    S::Error: WebResponseError<St, St::Error>,
    error::PayloadError: WebResponseError<St, St::Error>,
{
    type Res = WebResponse;
    /// The error type differs from the wrapped service's, see above.
    type Error = WebError<St, St::Error>;

    // Readiness is delegated to the wrapped service. Its error type is
    // `S::Error`, so it has to be converted with `WebError::from_err`.
    ntex::forward_ready!(St, service, WebError::from_err);
    ntex::forward_shutdown!(St, service);

    /// Called for every request that reaches this middleware.
    async fn call(
        &self,
        mut req: WebRequest,
        ctx: Ctx<'_, Self, St>,
    ) -> Result<Self::Res, Self::Error> {
        // 1. Read the whole request body.
        //
        // `take_payload()` moves the payload stream out of the request and
        // leaves an empty one in its place. The chunks are collected into one
        // buffer; a read error ends the request with that error.
        //
        // This keeps the whole body in memory. A real middleware should limit
        // its size, or inspect the chunks as they pass without buffering them.
        let mut body = BytesMut::new();
        let mut stream = req.take_payload();
        while let Some(chunk) = stream.next().await {
            body.extend_from_slice(&(chunk.map_err(WebError::from_err)?));
        }

        println!("request body: {:?}", body);

        // 2. Put the body back.
        //
        // The original stream is used up, so the collected bytes are wrapped
        // in a new single-chunk stream. Without this the handler, and any
        // extractor such as `web::types::Json`, would see an empty body.
        let body = body.freeze();
        let payload: PayloadStream = Box::pin(stream::once(async move { Ok(body) }));
        req.set_payload(Payload::Stream(payload));

        // 3. Call the next service in the chain, and convert its error.
        let res = ctx
            .call(&self.service, req)
            .await
            .map_err(WebError::from_err)?;

        // 4. The response is only logged, it is returned unchanged.
        println!("response: {:?}", res.headers());
        Ok(res)
    }
}
