use diesel::PgConnection;
use diesel::prelude::*;
use ntex::http::Payload;
use ntex::web::{self, FromRequest, HttpRequest, HttpResponse, error::BlockingError};
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

impl FromRequest<crate::AppState> for LoggedUser {
    type Error = ServiceError;

    async fn from_request(
        _: &crate::AppState,
        req: &HttpRequest,
        _: &mut Payload,
    ) -> Result<Self, Self::Error> {
        let id = req.get_identity();

        if let Some(identity) = id {
            serde_json::from_str::<LoggedUser>(&identity).map_err(|_| ServiceError::Unauthorized)
        } else {
            Err(ServiceError::Unauthorized.into())
        }
    }
}

pub async fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Ok().build()
}

pub async fn login(
    state: &crate::AppState,
    _: (),
    auth_data: web::types::Json<AuthData>,
    id: Identity,
) -> Result<HttpResponse, ServiceError> {
    let pool = state.st().clone();
    let res = web::block(move || query(auth_data.into_inner(), pool)).await;

    match res {
        Ok(user) => {
            let user_string = serde_json::to_string(&user).unwrap();
            id.remember(user_string);
            Ok(HttpResponse::Ok().build())
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
