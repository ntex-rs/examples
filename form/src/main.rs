use serde::{Deserialize, Serialize};

use ntex::web::{self, App, HttpRequest, HttpResponse, WebError, middleware};

type Error = WebError<AppState, web::DefaultError>;

#[derive(Clone)]
struct AppState {
    foo: String,
}

impl web::State for AppState {
    type Error = web::DefaultError;
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    web::server(async |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(app_config)
            .build_with(AppState {
                foo: "bar".to_string(),
            })
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("FORM"))?
    .run()
    .await
}

fn app_config(config: &mut web::ServiceConfig<AppState>) {
    config.service(web::scope("/").service((
        web::resource("/").route(web::get().to(index)),
        web::resource("/post1").route(web::post().to(handle_post_1)),
        web::resource("/post2").route(web::post().to_with_state(handle_post_2)),
        web::resource("/post3").route(web::post().to(handle_post_3)),
    )));
}

async fn index() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html")))
}

#[derive(Serialize, Deserialize)]
pub struct MyParams {
    name: String,
}

/// Handles a form submission.
async fn handle_post_1(params: web::types::Form<MyParams>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Your name is {}", params.name)))
}

/// Handles a form submission using application state.
async fn handle_post_2(
    state: &AppState,
    _: (),
    params: web::types::Form<MyParams>,
) -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain").body(format!(
        "Your name is {}, and in AppState I have foo: {}",
        params.name, state.foo
    ))
}

/// Handles a form submission with access to the request.
async fn handle_post_3(req: HttpRequest, params: web::types::Form<MyParams>) -> HttpResponse {
    println!("Handling POST request: {:?}", req);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Your name is {}", params.name))
}

#[cfg(test)]
mod tests {

    use super::*;

    use ntex::http::StatusCode;
    use ntex::http::body::{Body, ResponseBody};
    use ntex::http::header::{CONTENT_TYPE, HeaderValue};
    use ntex::web::test::{self, TestRequest};
    use ntex::web::types::Form;

    trait BodyTest {
        fn as_str(&self) -> &str;
    }

    impl BodyTest for ResponseBody<Body> {
        fn as_str(&self) -> &str {
            match self {
                ResponseBody::Body(b) => match b {
                    Body::Bytes(by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
                ResponseBody::Other(b) => match b {
                    Body::Bytes(by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
            }
        }
    }

    #[ntex::test]
    async fn index_unit_test() {
        let resp = index().await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html; charset=utf-8")
        );
        assert_eq!(resp.body().as_str(), include_str!("../static/form.html"));
    }

    #[ntex::test]
    async fn handle_post_1_unit_test() {
        let params = Form(MyParams {
            name: "John".to_string(),
        });
        let resp = handle_post_1(params).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(resp.body().as_str(), "Your name is John");
    }

    #[ntex::test]
    async fn handle_post_1_integration_test() {
        let app = test::init_service_st(
            AppState {
                foo: "bar".to_string(),
            },
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/post1")
            .form(&MyParams {
                name: "John".to_string(),
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(resp.body().as_str(), "Your name is John");
    }

    #[ntex::test]
    async fn handle_post_2_integration_test() {
        let app = test::init_service_st(
            AppState {
                foo: "bar".to_string(),
            },
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/post2")
            .form(&MyParams {
                name: "John".to_string(),
            })
            .to_request();
        let resp = app.call(req).await.unwrap();
        println!("R: {:?}", resp);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(
            resp.body().as_str(),
            "Your name is John, and in AppState I have foo: bar"
        );
    }

    #[ntex::test]
    async fn handle_post_3_unit_test() {
        let req = TestRequest::default().to_http_request();
        let params = Form(MyParams {
            name: "John".to_string(),
        });
        let result = handle_post_3(req.clone(), params).await;
        let resp = test::respond_to(result, &req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(resp.body().as_str(), "Your name is John");
    }

    #[ntex::test]
    async fn handle_post_3_integration_test() {
        let app = test::init_service_st(
            AppState {
                foo: "bar".to_string(),
            },
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/post3")
            .form(&MyParams {
                name: "John".to_string(),
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(resp.body().as_str(), "Your name is John");
    }
}
