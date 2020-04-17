use std::fmt;

use ntex::web::WebResponseError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error(anyhow::Error);

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

impl<E> From<E> for Error
where
    E: Into<anyhow::Error>,
{
    fn from(inner: E) -> Self {
        Self(inner.into())
    }
}

impl WebResponseError for Error {}
