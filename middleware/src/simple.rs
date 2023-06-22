use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::util::BoxFuture;
use ntex::web::{Error, WebRequest, WebResponse};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct SayHi;

// Middleware factory is `Middleware` trait from ntex-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S> Middleware<S> for SayHi {
    type Service = SayHiMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        SayHiMiddleware { service }
    }
}

pub struct SayHiMiddleware<S> {
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for SayHiMiddleware<S>
where
    Err: 'static,
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Response = WebResponse;
    type Error = Error;
    type Future<'f> = BoxFuture<'f, Result<Self::Response, Self::Error>> where S: 'f;

    ntex::forward_poll_ready!(service);
    ntex::forward_poll_shutdown!(service);

    fn call<'a>(
        &'a self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'a, Self>,
    ) -> Self::Future<'a> {
        println!("Hi from start. You requested: {}", req.path());

        Box::pin(async move {
            let res = ctx.call(&self.service, req).await?;

            println!("Hi from response");
            Ok(res)
        })
    }
}
