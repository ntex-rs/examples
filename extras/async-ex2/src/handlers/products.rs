use ntex::web::{self, DefaultError, HttpResponse, WebError};

use crate::common::{Part, Product};

type Error = WebError<(), DefaultError>;

pub async fn get_products(_query: web::types::Query<Option<Part>>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().build())
}

pub async fn add_product(_new_product: web::types::Json<Product>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().build())
}

pub async fn get_product_detail(_id: web::types::Path<String>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().build())
}

pub async fn remove_product(_id: web::types::Path<String>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().build())
}

#[cfg(test)]
mod tests {
    use ntex::http::{StatusCode, header};
    use ntex::web::{App, test};

    use crate::appconfig::config_app;

    /// STATEMENT: The configured product-creation route accepts a valid JSON product.
    ///
    /// HOW:
    /// - Initialize the real app with `config_app` and POST a product to `/products`.
    /// - Check that the request succeeds with HTTP 200.
    #[ntex::test]
    async fn test_add_product() {
        let app = test::init_service(App::new().configure(config_app)).await;

        let payload = r#"{"id":12345,"product_type":"fancy","name":"test"}"#.as_bytes();

        let req = test::TestRequest::post()
            .uri("/products")
            .header(header::CONTENT_TYPE, "application/json")
            .payload(payload)
            .to_request();

        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// STATEMENT: Product creation rejects malformed JSON before running the handler.
    ///
    /// HOW:
    /// - POST malformed JSON to `/products` using the real app configuration.
    /// - Check that extraction renders HTTP 400.
    #[ntex::test]
    async fn malformed_product_is_bad_request() {
        let app = test::init_service(App::new().configure(config_app)).await;
        let req = test::TestRequest::post()
            .uri("/products")
            .header(header::CONTENT_TYPE, "application/json")
            .payload("{")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// STATEMENT: Product-detail routing captures the product identifier.
    ///
    /// HOW:
    /// - GET `/products/42` through the real nested scope configuration.
    /// - Check HTTP 200 and the stub handler's empty response body.
    #[ntex::test]
    async fn product_detail_route_succeeds() {
        let app = test::init_service(App::new().configure(config_app)).await;
        let req = test::TestRequest::get().uri("/products/42").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(test::read_body(resp).await.is_empty());
    }
}
