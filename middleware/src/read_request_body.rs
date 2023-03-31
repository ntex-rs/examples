use std::rc::Rc;

use futures::stream::StreamExt;
use ntex::service::{Middleware, Service};
use ntex::util::{BoxFuture, BytesMut};
use ntex::web::{Error, ErrorRenderer, WebRequest, WebResponse};

pub struct Logging;

impl<S> Middleware<S> for Logging {
    type Service = LoggingMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        LoggingMiddleware {
            service: Rc::new(service),
        }
    }
}

pub struct LoggingMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: Rc<S>,
}

impl<S, Err> Service<WebRequest<Err>> for LoggingMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
    Err: ErrorRenderer,
{
    type Response = WebResponse;
    type Error = Error;
    type Future<'f> = BoxFuture<'f, Result<Self::Response, Self::Error>> where Self: 'f;

    ntex::forward_poll_ready!(service);

    fn call(&self, mut req: WebRequest<Err>) -> Self::Future<'_> {
        let svc = self.service.clone();

        Box::pin(async move {
            let mut body = BytesMut::new();
            let mut stream = req.take_payload();
            while let Some(chunk) = stream.next().await {
                body.extend_from_slice(&chunk?);
            }

            println!("request body: {:?}", body);
            let res = svc.call(req).await?;

            println!("response: {:?}", res.headers());
            Ok(res)
        })
    }
}
