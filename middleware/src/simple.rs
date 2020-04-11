use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::{ok, Ready};
use futures::Future;
use ntex::web::dev::{WebRequest, WebResponse};
use ntex::web::Error;
use ntex::{Service, Transform};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct SayHi;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, Err> Transform<S> for SayHi
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse, Error = Error>,
    S::Future: 'static,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse;
    type Error = Error;
    type InitError = ();
    type Transform = SayHiMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SayHiMiddleware { service })
    }
}

pub struct SayHiMiddleware<S> {
    service: S,
}

impl<S, Err> Service for SayHiMiddleware<S>
where
    S: Service<Request = WebRequest<Err>, Response = WebResponse, Error = Error>,
    S::Future: 'static,
{
    type Request = WebRequest<Err>;
    type Response = WebResponse;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: Self::Request) -> Self::Future {
        println!("Hi from start. You requested: {}", req.path());

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            println!("Hi from response");
            Ok(res)
        })
    }
}
