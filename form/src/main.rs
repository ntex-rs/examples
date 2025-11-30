use serde::{Deserialize, Serialize};

use ntex::web::{self, middleware, App, Error, HttpRequest, HttpResponse};

struct AppState {
    foo: String,
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    web::server(async || {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(app_config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/")
            .state(AppState {
                foo: "bar".to_string(),
            })
            .service((
                web::resource("/").route(web::get().to(index)),
                web::resource("/post1").route(web::post().to(handle_post_1)),
                web::resource("/post2").route(web::post().to(handle_post_2)),
                web::resource("/post3").route(web::post().to(handle_post_3)),
            )),
    );
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

/// Simple handle POST request
async fn handle_post_1(
    params: web::types::Form<MyParams>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Your name is {}", params.name)))
}

/// State and POST Params
async fn handle_post_2(
    state: web::types::State<AppState>,
    params: web::types::Form<MyParams>,
) -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain").body(format!(
        "Your name is {}, and in AppState I have foo: {}",
        params.name, state.foo
    ))
}

/// Request and POST Params
async fn handle_post_3(
    req: HttpRequest,
    params: web::types::Form<MyParams>,
) -> HttpResponse {
    println!("Handling POST request: {:?}", req);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Your name is {}", params.name))
}

#[cfg(test)]
mod tests {

    use super::*;

    use ntex::http::body::{Body, ResponseBody};
    use ntex::http::header::{HeaderValue, CONTENT_TYPE};
    use ntex::http::StatusCode;
    use ntex::web::test::{self, TestRequest};
    use ntex::web::types::Form;

    trait BodyTest {
        fn as_str(&self) -> &str;
    }

    impl BodyTest for ResponseBody<Body> {
        fn as_str(&self) -> &str {
            match self {
                ResponseBody::Body(ref b) => match b {
                    Body::Bytes(ref by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
                ResponseBody::Other(ref b) => match b {
                    Body::Bytes(ref by) => std::str::from_utf8(by).unwrap(),
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
        let app = test::init_service(App::new().configure(app_config)).await;
        let req = test::TestRequest::post()
            .uri("/post1")
            .set_form(&MyParams {
                name: "John".to_string(),
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(resp.response().body().as_str(), "Your name is John");
    }

    // #[ntex::test]
    // async fn handle_post_2_unit_test() {
    // let app_state = AppState {
    //     foo: "bar".to_string(),
    // };

    // let req = TestRequest::default().state(app_state).to_srv_request();
    // let data = req.app_state::<AppState>().unwrap();
    // let params = Form(MyParams {
    //     name: "John".to_string(),
    // });
    // let resp = handle_post_2(data.clone(), params).await;

    // assert_eq!(resp.status(), StatusCode::OK);
    // assert_eq!(
    //     resp.headers().get(CONTENT_TYPE).unwrap(),
    //     HeaderValue::from_static("text/plain")
    // );
    // assert_eq!(
    //     resp.body().as_str(),
    //     "Your name is John, and in AppState I have foo: bar"
    // );
    // }

    #[ntex::test]
    async fn handle_post_2_integration_test() {
        let app = test::init_service(App::new().configure(app_config)).await;
        let req = test::TestRequest::post()
            .uri("/post2")
            .set_form(&MyParams {
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
            resp.response().body().as_str(),
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
        let app = test::init_service(App::new().configure(app_config)).await;
        let req = test::TestRequest::post()
            .uri("/post3")
            .set_form(&MyParams {
                name: "John".to_string(),
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(resp.response().body().as_str(), "Your name is John");
    }
}
