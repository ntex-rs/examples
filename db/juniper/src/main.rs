//! Serve a demonstration GraphQL schema with Juniper and ntex.
//!
//! `AppState` shares the schema from `src/schema.rs`, and `web::block`
//! executes JSON GraphQL requests away from server workers. The query returns
//! fixed data and the mutation echoes its input without persistence.
//! Run with `cargo run` and open <http://127.0.0.1:8080/graphiql>.

use std::{io, sync::Arc};

use juniper::http::GraphQLRequest;
use juniper::http::graphiql::graphiql_source;
use ntex::web::{self, App, HttpResponse, WebError, middleware, types};

mod schema;

use crate::schema::{Schema, create_schema};

type Error = WebError<AppState>;

#[derive(Clone)]
struct AppState {
    schema: Arc<Schema>,
}

impl web::State for AppState {
    type Error = web::DefaultError;
}

#[web::get("/graphiql", state=AppState)]
async fn graphiql() -> HttpResponse {
    let html = graphiql_source("http://127.0.0.1:8080/graphql", None);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

async fn graphql(
    st: &AppState,
    _: (),
    data: types::Json<GraphQLRequest>,
) -> Result<HttpResponse, Error> {
    let st = st.clone();
    let user = web::block(move || {
        let res = data.execute_sync(&st.schema, &());
        serde_json::to_string(&res)
    })
    .await
    .map_err(Error::from_err)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(user))
}

fn config(cfg: &mut web::ServiceConfig<AppState>) {
    cfg.service((
        web::resource("/graphql").route(web::post().to_with_state(graphql)),
        graphiql,
    ));
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    // Create Juniper schema
    let schema = Arc::new(create_schema());

    // Start the HTTP server.
    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(config)
            .build_with(AppState {
                schema: schema.clone(),
            })
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("J"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test::{self, TestRequest};
    use serde_json::json;

    fn state() -> AppState {
        AppState {
            schema: Arc::new(create_schema()),
        }
    }

    /// STATEMENT: The GraphQL endpoint executes a query against the example schema.
    ///
    /// HOW:
    /// - Initialize the real routes with `create_schema` and submit a JSON `human` query.
    /// - Check the status, JSON content type, and the complete returned human data.
    #[ntex::test]
    async fn human_query_returns_schema_data() {
        let app = test::init_service_st(state(), App::new().configure(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/graphql")
                .json(&json!({"query": "{ human(id: \"42\") { id name appearsIn homePlanet } }"}))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert_eq!(
            body,
            json!({"data": {"human": {
                "id": "42", "name": "Luke", "appearsIn": ["NEW_HOPE"], "homePlanet": "Mars"
            }}})
        );
    }

    /// STATEMENT: The GraphiQL route serves an HTML interface targeting the GraphQL endpoint.
    ///
    /// HOW:
    /// - Initialize the real routes and request `GET /graphiql`.
    /// - Check the status, HTML content type, interface name, and GraphQL URL in the body.
    #[ntex::test]
    async fn graphiql_returns_html() {
        let app = test::init_service_st(state(), App::new().configure(config)).await;
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
        assert!(html.contains("http://127.0.0.1:8080/graphql"));
    }

    /// STATEMENT: An unknown GraphQL field produces a GraphQL error response.
    ///
    /// HOW:
    /// - Submit a JSON query containing `unknownField` through the real routes.
    /// - Check the HTTP status and JSON content type, then verify the reported field error.
    #[ntex::test]
    async fn unknown_field_returns_graphql_error() {
        let app = test::init_service_st(state(), App::new().configure(config)).await;
        let response = test::call_service(
            &app,
            TestRequest::post()
                .uri("/graphql")
                .json(&json!({"query": "{ unknownField }"}))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: serde_json::Value =
            serde_json::from_slice(&test::read_body(response).await).unwrap();
        assert!(
            body["errors"][0]["message"]
                .as_str()
                .unwrap()
                .contains("unknownField")
        );
    }
}
