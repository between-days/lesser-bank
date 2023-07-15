use std::fmt;

use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};

#[derive(Debug)]
pub enum AccountsApiError {
    NotFound,
    InternalError,
}

impl fmt::Display for AccountsApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

impl error::ResponseError for AccountsApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(match self {
                Self::NotFound => "Entity not found",
                Self::InternalError => "Internal server error",
            })
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            AccountsApiError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AccountsApiError::NotFound => StatusCode::NOT_FOUND,
        }
    }
}