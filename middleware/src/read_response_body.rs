use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::future::{ok, Ready};
use ntex::http::body::{BodySize, MessageBody, ResponseBody};
use ntex::web::dev::{WebRequest, WebResponse};
use ntex::web::Error;
use ntex::{Service, Transform};

pub struct Logging;

impl<S: 'static, B, Err> Transform<S> for Logging
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse<B>, Error = Error>,
    B: MessageBody + 'static,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse<BodyLogger<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggingMiddleware<S, Err>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggingMiddleware {
            service,
            _t: PhantomData,
        })
    }
}

pub struct LoggingMiddleware<S, Err> {
    service: S,
    _t: PhantomData<Err>,
}

impl<S, B, Err> Service for LoggingMiddleware<S, Err>
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse<BodyLogger<B>>;
    type Error = Error;
    type Future = WrapperStream<S, B, Err>;

    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: Self::Request) -> Self::Future {
        WrapperStream {
            fut: self.service.call(req),
            _t: PhantomData,
        }
    }
}

#[pin_project::pin_project]
pub struct WrapperStream<S, B, Err>
where
    B: MessageBody,
    S: Service,
{
    #[pin]
    fut: S::Future,
    _t: PhantomData<(B, Err)>,
}

impl<S, B, Err> Future for WrapperStream<S, B, Err>
where
    B: MessageBody,
    S: Service<Request = WebRequest<Err>, Response = WebResponse<B>, Error = Error>,
{
    type Output = Result<WebResponse<BodyLogger<B>>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = futures::ready!(self.project().fut.poll(cx));

        Poll::Ready(res.map(|res| {
            res.map_body(move |_, body| {
                ResponseBody::Body(BodyLogger {
                    body,
                    body_accum: BytesMut::new(),
                })
            })
        }))
    }
}

pub struct BodyLogger<B> {
    body: ResponseBody<B>,
    body_accum: BytesMut,
}

impl<B> Drop for BodyLogger<B> {
    fn drop(&mut self) {
        println!("response body: {:?}", self.body_accum);
    }
}

impl<B: MessageBody> MessageBody for BodyLogger<B> {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Bytes, Box<dyn std::error::Error>>>> {
        match self.body.poll_next_chunk(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.body_accum.extend_from_slice(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
