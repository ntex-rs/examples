use ntex::service::{Ctx, Middleware, Service};
use ntex::web::{State, WebRequest, WebResponse};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct SayHi;

// Middleware factory is `Middleware` trait from ntex-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, St> Middleware<S, St> for SayHi {
    type Service = SayHiMiddleware<S>;

    fn create(&self, _: &St, service: S) -> Self::Service {
        SayHiMiddleware { service }
    }
}

pub struct SayHiMiddleware<S> {
    service: S,
}

impl<S, St: State> Service<St, WebRequest> for SayHiMiddleware<S>
where
    S: Service<St, WebRequest, Res = WebResponse>,
{
    type Res = WebResponse;
    type Error = S::Error;

    ntex::forward_ready!(St, service);
    ntex::forward_shutdown!(St, service);

    async fn call(
        &self,
        req: WebRequest,
        ctx: Ctx<'_, Self, St>,
    ) -> Result<Self::Res, Self::Error> {
        println!("Hi from start. You requested: {}", req.path());

        let res = ctx.call(&self.service, req).await?;
        println!("Hi from response");
        Ok(res)
    }
}
