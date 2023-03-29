use diesel::prelude::*;
use diesel::PgConnection;
use futures::future::{ready, Ready};
use ntex::http::Payload;
use ntex::web::{
    self, error::BlockingError, Error, FromRequest, HttpRequest, HttpResponse,
};
use ntex_identity::{Identity, RequestIdentity};
use serde::Deserialize;

use crate::errors::ServiceError;
use crate::models::{Pool, SlimUser, User};
use crate::utils::verify;

#[derive(Debug, Deserialize)]
pub struct AuthData {
    pub email: String,
    pub password: String,
}

// we need the same data
// simple aliasing makes the intentions clear and its more readable
pub type LoggedUser = SlimUser;

impl<Err> FromRequest<Err> for LoggedUser {
    type Error = Error;
    type Future = Ready<Result<LoggedUser, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let id = req.get_identity();

        ready(if let Some(identity) = id {
            serde_json::from_str::<LoggedUser>(&identity).map_err(From::from)
        } else {
            Err(ServiceError::Unauthorized.into())
        })
    }
}

pub async fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Ok().finish()
}

pub async fn login(
    auth_data: web::types::Json<AuthData>,
    id: Identity,
    pool: web::types::State<Pool>,
) -> Result<HttpResponse, ServiceError> {
    let pool = (*pool).clone();
    let res = web::block(move || query(auth_data.into_inner(), pool)).await;

    match res {
        Ok(user) => {
            let user_string = serde_json::to_string(&user).unwrap();
            id.remember(user_string);
            Ok(HttpResponse::Ok().finish())
        }
        Err(err) => match err {
            BlockingError::Error(service_error) => Err(service_error),
            BlockingError::Canceled => Err(ServiceError::InternalServerError),
        },
    }
}

pub async fn get_me(logged_user: LoggedUser) -> HttpResponse {
    HttpResponse::Ok().json(&logged_user)
}
/// Diesel query
fn query(auth_data: AuthData, pool: Pool) -> Result<SlimUser, ServiceError> {
    use crate::schema::users::dsl::{email, users};
    let conn: &PgConnection = &pool.get().unwrap();
    let mut items = users
        .filter(email.eq(&auth_data.email))
        .load::<User>(conn)?;

    if let Some(user) = items.pop() {
        if let Ok(matching) = verify(&user.hash, &auth_data.password) {
            if matching {
                return Ok(user.into());
            }
        }
    }
    Err(ServiceError::Unauthorized)
}
