use ntex::web::{self, App, HttpRequest, middleware};

async fn index(req: HttpRequest) -> &'static str {
    println!("REQ: {:?}", req);
    "Hello world!"
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .service((
                web::resource("/index.html").to(|| async { "Hello world!" }),
                web::resource("/").to(index),
            ))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("HW"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::util::Bytes;
    use ntex::web::{App, Error, test};
    use ntex::{http, web};

    #[ntex::test]
    async fn test_index() -> Result<(), Error> {
        let app = App::new().route("/", web::get().to(index));
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let bytes = test::read_body(resp).await;

        assert_eq!(bytes, Bytes::from(r##"Hello world!"##));

        Ok(())
    }
}
