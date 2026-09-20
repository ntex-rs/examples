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
}
