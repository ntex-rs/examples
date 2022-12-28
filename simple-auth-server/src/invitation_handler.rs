use diesel::{prelude::*, PgConnection};
use ntex::web::{self, error::BlockingError, HttpResponse};
use serde::Deserialize;

use crate::email_service::send_invitation;
use crate::errors::ServiceError;
use crate::models::{Invitation, Pool};

#[derive(Deserialize)]
pub struct InvitationData {
    pub email: String,
}

pub async fn post_invitation(
    invitation_data: web::types::Json<InvitationData>,
    pool: web::types::State<Pool>,
) -> Result<HttpResponse, ServiceError> {
    // run diesel blocking code
    let pool = (&*pool).clone();
    let res =
        web::block(move || create_invitation(invitation_data.into_inner().email, pool))
            .await;

    match res {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => match err {
            BlockingError::Error(service_error) => Err(service_error),
            BlockingError::Canceled => Err(ServiceError::InternalServerError),
        },
    }
}

fn create_invitation(
    eml: String,
    pool: Pool,
) -> Result<(), crate::errors::ServiceError> {
    let invitation = dbg!(query(eml, pool)?);
    send_invitation(&invitation)
}

/// Diesel query
fn query(eml: String, pool: Pool) -> Result<Invitation, crate::errors::ServiceError> {
    use crate::schema::invitations::dsl::invitations;

    let new_invitation: Invitation = eml.into();
    let conn: &PgConnection = &pool.get().unwrap();

    let inserted_invitation = diesel::insert_into(invitations)
        .values(&new_invitation)
        .get_result(conn)?;

    Ok(inserted_invitation)
}
