use ntex::http;
use ntex::service::{Ctx, Middleware, Service};
use ntex::web::{HttpResponse, WebRequest, WebResponse};

pub struct CheckLogin;

impl<S, St> Middleware<S, St> for CheckLogin {
    type Service = CheckLoginMiddleware<S>;

    fn create(&self, _: &St, service: S) -> Self::Service {
        CheckLoginMiddleware { service }
    }
}

pub struct CheckLoginMiddleware<S> {
    service: S,
}

impl<S, St> Service<St, WebRequest> for CheckLoginMiddleware<S>
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
        // We only need to hook into the `start` for this middleware.

        let is_logged_in = false; // Change this to see the change in outcome in the browser

        if is_logged_in {
            ctx.call(&self.service, req).await
        } else {
            // Don't forward to /login if we are already on /login
            if req.path() == "/login" {
                ctx.call(&self.service, req).await
            } else {
                Ok(req.into_response(
                    HttpResponse::Found()
                        .header(http::header::LOCATION, "/login")
                        .build()
                        .into_body(),
                ))
            }
        }
    }
}
