use futures::stream::StreamExt;
use ntex::service::{Ctx, Middleware, Service};
use ntex::web::{State, WebError, WebRequest, WebResponse, WebResponseError};
use ntex::{http::error, util::BytesMut};

pub struct Logging;

impl<S, St> Middleware<S, St> for Logging {
    type Service = LoggingMiddleware<S>;

    fn create(&self, _: &St, service: S) -> Self::Service {
        LoggingMiddleware { service }
    }
}

pub struct LoggingMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: S,
}

impl<S, St> Service<St, WebRequest> for LoggingMiddleware<S>
where
    St: State,
    S: Service<St, WebRequest, Res = WebResponse> + 'static,
    S::Error: WebResponseError<St, St::Error>,
    error::PayloadError: WebResponseError<St, St::Error>,
{
    type Res = WebResponse;
    type Error = WebError<St, St::Error>;

    ntex::forward_ready!(St, service, WebError::from_err);
    ntex::forward_shutdown!(St, service);

    async fn call(
        &self,
        mut req: WebRequest,
        ctx: Ctx<'_, Self, St>,
    ) -> Result<Self::Res, Self::Error> {
        let mut body = BytesMut::new();
        let mut stream = req.take_payload();
        while let Some(chunk) = stream.next().await {
            body.extend_from_slice(&(chunk.map_err(WebError::from_err)?));
        }

        println!("request body: {:?}", body);
        let res = ctx
            .call(&self.service, req)
            .await
            .map_err(WebError::from_err)?;

        println!("response: {:?}", res.headers());
        Ok(res)
    }
}
