use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web::{Error, WebRequest, WebResponse};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct SayHi;

// Middleware factory is `Middleware` trait from ntex-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, C> Middleware<S, C> for SayHi {
    type Service = SayHiMiddleware<S>;

    fn create(&self, service: S, _: C) -> Self::Service {
        SayHiMiddleware { service }
    }
}

pub struct SayHiMiddleware<S> {
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for SayHiMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_ready!(service);
    ntex::forward_poll!(service);
    ntex::forward_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        println!("Hi from start. You requested: {}", req.path());

        let res = ctx.call(&self.service, req).await?;
        println!("Hi from response");
        Ok(res)
    }
}
