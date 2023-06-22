use futures::stream::StreamExt;
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::util::{BoxFuture, BytesMut};
use ntex::web::{Error, ErrorRenderer, WebRequest, WebResponse};

pub struct Logging;

impl<S> Middleware<S> for Logging {
    type Service = LoggingMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
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
    type Future<'f> = BoxFuture<'f, Result<Self::Response, Self::Error>> where S: 'f;

    ntex::forward_poll_ready!(service);
    ntex::forward_poll_shutdown!(service);

    fn call<'a>(
        &'a self,
        mut req: WebRequest<Err>,
        ctx: ServiceCtx<'a, Self>,
    ) -> Self::Future<'a> {
        Box::pin(async move {
            let mut body = BytesMut::new();
            let mut stream = req.take_payload();
            while let Some(chunk) = stream.next().await {
                body.extend_from_slice(&chunk?);
            }

            println!("request body: {:?}", body);
            let res = ctx.call(&self.service, req).await?;

            println!("response: {:?}", res.headers());
            Ok(res)
        })
    }
}
