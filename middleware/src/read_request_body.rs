use futures::stream::StreamExt;
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::util::BytesMut;
use ntex::web::{Error, ErrorRenderer, WebRequest, WebResponse};

pub struct Logging;

impl<S, C> Middleware<S, C> for Logging {
    type Service = LoggingMiddleware<S>;

    fn create(&self, service: S, _: C) -> Self::Service {
        LoggingMiddleware { service }
    }
}

pub struct LoggingMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for LoggingMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error> + 'static,
    Err: ErrorRenderer + 'static,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_ready!(service);
    ntex::forward_shutdown!(service);

    async fn call(
        &self,
        mut req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        let mut body = BytesMut::new();
        let mut stream = req.take_payload();
        while let Some(chunk) = stream.next().await {
            body.extend_from_slice(&chunk?);
        }

        println!("request body: {:?}", body);
        let res = ctx.call(&self.service, req).await?;

        println!("response: {:?}", res.headers());
        Ok(res)
    }
}
