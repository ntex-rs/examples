use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::future::{ok, Ready};
use ntex::http::body::{Body, BodySize, MessageBody, ResponseBody};
use ntex::web::dev::{WebRequest, WebResponse};
use ntex::web::Error;
use ntex::{Service, Transform};

pub struct Logging;

impl<S: 'static, Err> Transform<S> for Logging
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse;
    type Error = Error;
    type InitError = ();
    type Transform = LoggingMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggingMiddleware { service })
    }
}

pub struct LoggingMiddleware<S> {
    service: S,
}

impl<S, Err> Service for LoggingMiddleware<S>
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse;
    type Error = Error;
    type Future = WrapperStream<S>;

    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: Self::Request) -> Self::Future {
        WrapperStream {
            fut: self.service.call(req),
        }
    }
}

#[pin_project::pin_project]
pub struct WrapperStream<S>
where
    S: Service,
{
    #[pin]
    fut: S::Future,
}

impl<S, Err> Future for WrapperStream<S>
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Output = Result<WebResponse, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = futures::ready!(self.project().fut.poll(cx));

        Poll::Ready(res.map(|res| {
            res.map_body(move |_, body| {
                Body::from_message(BodyLogger {
                    body,
                    body_accum: BytesMut::new(),
                })
                .into()
            })
        }))
    }
}

pub struct BodyLogger {
    body: ResponseBody<Body>,
    body_accum: BytesMut,
}

impl Drop for BodyLogger {
    fn drop(&mut self) {
        println!("response body: {:?}", self.body_accum);
    }
}

impl MessageBody for BodyLogger {
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
