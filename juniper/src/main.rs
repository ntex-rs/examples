//! ntex Juniper example
//!
//! A simple example of integrating Juniper with ntex.
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
    let html = graphiql_source("http://127.0.0.1:8080/graphql");
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
        let res = data.execute(&st.schema, &());
        serde_json::to_string(&res)
    })
    .await
    .map_err(Error::from_err)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(user))
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
            .service((
                web::resource("/graphql").route(web::post().to_with_state(graphql)),
                graphiql,
            ))
            .build_with(AppState {
                schema: schema.clone(),
            })
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("J"))?
    .run()
    .await
}
