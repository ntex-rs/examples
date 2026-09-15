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
    st: types::State<AppState>,
    data: types::Json<GraphQLRequest>,
) -> Result<HttpResponse, WebError<AppState>> {
    let schema = st.schema.clone();
    let ctx = Context {
        dbpool: st.pool.clone(),
    };
    let res = web::block(move || {
        let res = data.execute(&schema, &ctx);
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
        .body(graphiql_source("/graphql"))
}

pub fn register(config: &mut web::ServiceConfig<AppState>) {
    config
        .route("/graphql", web::post().to(graphql))
        .route("/graphiql", web::get().to(graphql_playground));
}
