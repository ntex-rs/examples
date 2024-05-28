use std::{task::Context, task::Poll};

use ntex::http::body::{Body, BodySize, MessageBody, ResponseBody};
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::util::{Bytes, BytesMut};
use ntex::web::{Error, WebRequest, WebResponse};

pub struct Logging;

impl<S> Middleware<S> for Logging {
    type Service = LoggingMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        LoggingMiddleware { service }
    }
}

pub struct LoggingMiddleware<S> {
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for LoggingMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_ready!(service);
    ntex::forward_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<WebResponse, Error> {
        ctx.call(&self.service, req).await.map(|res| {
            res.map_body(move |_, body| {
                Body::from_message(BodyLogger {
                    body,
                    body_accum: BytesMut::new(),
                })
                .into()
            })
        })
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
