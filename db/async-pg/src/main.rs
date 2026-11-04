//! Insert users asynchronously into PostgreSQL with ntex.
//!
//! `types::Json` extracts user data, `deadpool_postgres` pools connections,
//! and `tokio_pg_mapper` maps returned rows. The inline modules show
//! configuration, database access, handlers, and error responses.
//! Set up PostgreSQL and the local `.env` as described in the README.

mod config {
    pub use ::config::ConfigError;

    #[derive(serde::Deserialize)]
    pub struct Config {
        pub server_addr: String,
        pub pg: deadpool_postgres::Config,
    }

    impl Config {
        pub fn from_env() -> Result<Self, ConfigError> {
            ::config::Config::builder()
                .add_source(::config::Environment::default())
                .build()?
                .try_deserialize()
        }
    }
}

mod models {
    use serde::{Deserialize, Serialize};
    use tokio_pg_mapper_derive::PostgresMapper;

    #[derive(Deserialize, PostgresMapper, Serialize)]
    #[pg_mapper(table = "users")] // singular 'user' is a keyword..
    pub struct User {
        pub email: String,
        pub first_name: String,
        pub last_name: String,
        pub username: String,
    }
}

mod errors {
    use deadpool_postgres::PoolError;
    use derive_more::{Display, From};
    use ntex::web::{DefaultError, HttpResponse, WebResponseError};
    use tokio_pg_mapper::Error as PGMError;
    use tokio_postgres::error::Error as PGError;

    #[derive(Display, From, Debug)]
    pub enum MyError {
        NotFound,
        PGError(PGError),
        PGMError(PGMError),
        PoolError(PoolError),
    }
    impl std::error::Error for MyError {}

    impl<St> WebResponseError<St, DefaultError> for MyError {
        fn error_response(&self, _: &St) -> HttpResponse {
            match *self {
                MyError::NotFound => HttpResponse::NotFound().build(),
                MyError::PoolError(ref err) => {
                    HttpResponse::InternalServerError().body(err.to_string())
                }
                _ => HttpResponse::InternalServerError().build(),
            }
        }
    }
}

mod db {
    use deadpool_postgres::Client;
    use tokio_pg_mapper::FromTokioPostgresRow;

    use crate::{errors::MyError, models::User};

    pub async fn add_user(client: &Client, user_info: User) -> Result<User, MyError> {
        let _stmt = include_str!("../sql/add_user.sql");
        let _stmt = _stmt.replace("$table_fields", &User::sql_table_fields());
        let stmt = client.prepare(&_stmt).await.unwrap();

        client
            .query(
                &stmt,
                &[
                    &user_info.email,
                    &user_info.first_name,
                    &user_info.last_name,
                    &user_info.username,
                ],
            )
            .await?
            .iter()
            .map(|row| User::from_row_ref(row).unwrap())
            .collect::<Vec<User>>()
            .pop()
            .ok_or(MyError::NotFound) // more applicable for SELECTs
    }
}

mod handlers {
    use deadpool_postgres::{Client, Pool};
    use ntex::web::{self, DefaultError, HttpResponse, WebError, types};

    use crate::{db, errors::MyError, models::User};

    type Error = WebError<HttpState, DefaultError>;

    #[derive(Clone)]
    pub struct HttpState {
        pub pool: Pool,
    }

    impl web::State for HttpState {
        type Error = web::DefaultError;
    }

    pub async fn add_user(
        st: &HttpState,
        _: (),
        user: types::Json<User>,
    ) -> Result<HttpResponse, Error> {
        let user_info: User = user.into_inner();

        let client: Client = st
            .pool
            .get()
            .await
            .map_err(|e| Error::from_err(MyError::PoolError(e)))?;

        let new_user = db::add_user(&client, user_info)
            .await
            .map_err(Error::from_err)?;

        Ok(HttpResponse::Ok().json(&new_user))
    }
}

use deadpool_postgres::Runtime;
use dotenv::dotenv;
use handlers::{HttpState, add_user};
use ntex::web::{self, App};
use tokio_postgres::NoTls;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config = crate::config::Config::from_env().unwrap();
    let pool = config.pg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();

    let server = web::server(async move |_| {
        App::new()
            .service(web::resource("/users").route(web::post().to_with_state(add_user)))
            .build_with(HttpState { pool: pool.clone() })
    })
    .bind(config.server_addr.clone(), ntex::SharedCfg::new("PG"))?
    .run();
    println!("Server running at http://{}/", config.server_addr);

    server.await
}

#[cfg(test)]
mod tests {
    use super::errors::MyError;
    use deadpool_postgres::PoolError;
    use ntex::http::StatusCode;
    use ntex::web::{WebResponseError, test};

    /// STATEMENT: A missing database result is rendered as an empty HTTP not-found response.
    ///
    /// HOW:
    /// - Render `MyError::NotFound` using its real `WebResponseError` implementation.
    /// - Check for `404` and an empty response body without connecting to PostgreSQL.
    #[ntex::test]
    async fn not_found_error_returns_404() {
        let response = MyError::NotFound.error_response(&());

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
    }

    /// STATEMENT: A closed connection pool is rendered as a server error with its diagnostic.
    ///
    /// HOW:
    /// - Wrap `PoolError::Closed` in `MyError` and render its actual HTTP response.
    /// - Check for `500` and the pool diagnostic without creating a PostgreSQL connection.
    #[ntex::test]
    async fn closed_pool_error_returns_diagnostic() {
        let expected = PoolError::Closed.to_string();
        let response = MyError::PoolError(PoolError::Closed).error_response(&());

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(test::read_body(response).await, expected);
    }
}
