use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use bytes::BytesMut;
use futures::future::{ok, Future, Ready};
use futures::stream::StreamExt;
use ntex::web::dev::{WebRequest, WebResponse};
use ntex::web::{Error, ErrorRenderer};
use ntex::{Service, Transform};

pub struct Logging;

impl<S: 'static, B, Err> Transform<S> for Logging
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    Err: ErrorRenderer,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggingMiddleware<S, Err>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggingMiddleware {
            service: Rc::new(service),
            _t: PhantomData,
        })
    }
}

pub struct LoggingMiddleware<S, Err> {
    // This is special: We need this to avoid lifetime issues.
    service: Rc<S>,
    _t: PhantomData<Err>,
}

impl<S, B, Err> Service for LoggingMiddleware<S, Err>
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: 'static,
    Err: ErrorRenderer,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, mut req: WebRequest<Err>) -> Self::Future {
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
