use ntex::web::{self, Error, HttpResponse};

use crate::common::{Part, Product};

pub async fn get_parts(
    _query: web::types::Query<Option<Part>>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn add_part(
    _new_part: web::types::Json<Product>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_part_detail(
    _id: web::types::Path<String>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn remove_part(_id: web::types::Path<String>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}
