use derive_more::Display;
use diesel::result::{DatabaseErrorKind, Error as DBError};
use ntex::web::{DefaultError, HttpResponse, WebResponseError};
use std::convert::From;
use uuid::Error as ParseError;

#[derive(Debug, Display)]
pub enum ServiceError {
    #[display("Internal Server Error")]
    InternalServerError,

    #[display("BadRequest: {_0}")]
    BadRequest(String),

    #[display("Unauthorized")]
    Unauthorized,
}

impl std::error::Error for ServiceError {}

// Convert application errors into appropriate HTTP responses.
impl WebResponseError<crate::AppState, DefaultError> for ServiceError {
    fn error_response(&self, _: &crate::AppState) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => {
                HttpResponse::InternalServerError().json(&"Internal Server Error, Please try later")
            }
            ServiceError::BadRequest(message) => HttpResponse::BadRequest().json(message),
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json(&"Unauthorized"),
        }
    }
}

// Return a clear client error when an invalid UUID is provided.
impl From<ParseError> for ServiceError {
    fn from(_: ParseError) -> ServiceError {
        ServiceError::BadRequest("Invalid UUID".into())
    }
}

impl From<DBError> for ServiceError {
    fn from(error: DBError) -> ServiceError {
        // Right now we just care about UniqueViolation from diesel
        // But this would be helpful to easily map errors as our app grows
        match error {
            DBError::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return ServiceError::BadRequest(message);
                }
                ServiceError::InternalServerError
            }
            _ => ServiceError::InternalServerError,
        }
    }
}
