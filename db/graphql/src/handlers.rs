use std::sync::Arc;

use juniper::http::GraphQLRequest;
use juniper::http::graphiql::graphiql_source;
use ntex::web::{self, DefaultError, HttpResponse, WebError, types};

use crate::db::Pool;
use crate::schemas::root::{Context, Schema};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub schema: Arc<Schema>,
}

impl web::State for AppState {
    type Error = DefaultError;
}

pub async fn graphql(
    st: &AppState,
    _: (),
    data: types::Json<GraphQLRequest>,
) -> Result<HttpResponse, WebError<AppState>> {
    let schema = st.schema.clone();
    let ctx = Context {
        dbpool: st.pool.clone(),
    };
    let res = web::block(move || {
        let res = data.execute_sync(&schema, &ctx);
        serde_json::to_string(&res)
    })
    .await
    .map_err(WebError::from_err)?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(res))
}

pub async fn graphql_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(graphiql_source("/graphql", None))
}

pub fn register(config: &mut web::ServiceConfig<AppState>) {
    config
        .route("/graphql", web::post().to_with_state(graphql))
        .route("/graphiql", web::get().to(graphql_playground));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::root::create_schema;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};
    use r2d2_mysql::{MySqlConnectionManager, mysql::OptsBuilder};

    fn state() -> AppState {
        // No idle connections: these routes must not contact MySQL.
        let pool = Pool::builder()
            .max_size(1)
            .min_idle(Some(0))
            .build_unchecked(MySqlConnectionManager::new(OptsBuilder::default()));
        AppState {
            pool,
            schema: Arc::new(create_schema()),
        }
    }

    /// STATEMENT: The GraphiQL page is available without a MySQL connection.
    ///
    /// HOW:
    /// - Initialize the real `register` routes with a pool that opens no connections.
    /// - Request `GET /graphiql` and check the status, HTML content type, interface, and endpoint.
    #[ntex::test]
    async fn graphiql_returns_html_without_database() {
        let app = test::init_service_st(state(), web::App::new().configure(register)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/graphiql").to_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        let body = test::read_body(response).await;
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("GraphiQL"));
        assert!(html.contains("/graphql"));
    }

    /// STATEMENT: Invalid JSON is rejected before GraphQL attempts to access MySQL.
    ///
    /// HOW:
    /// - Initialize the real routes with a pool that opens no connections.
    /// - Send malformed JSON to `POST /graphql` and check for `400` and a JSON decoding error.
    #[ntex::test]
    async fn malformed_graphql_json_is_rejected() {
        let app = test::init_service_st(state(), web::App::new().configure(register)).await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/graphql")
                .header(header::CONTENT_TYPE, "application/json")
                .payload("{")
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = test::read_body(response).await;
        assert!(
            std::str::from_utf8(&body)
                .unwrap()
                .to_lowercase()
                .contains("json")
        );
    }
}
