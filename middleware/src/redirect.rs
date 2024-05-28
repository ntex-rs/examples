use ntex::http;
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web::{Error, ErrorRenderer, HttpResponse, WebRequest, WebResponse};

pub struct CheckLogin;

impl<S> Middleware<S> for CheckLogin {
    type Service = CheckLoginMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        CheckLoginMiddleware { service }
    }
}

pub struct CheckLoginMiddleware<S> {
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for CheckLoginMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
    Err: ErrorRenderer,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_ready!(service);
    ntex::forward_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
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
                        .finish()
                        .into_body(),
                ))
            }
        }
    }
}
