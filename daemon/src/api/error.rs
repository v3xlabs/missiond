use poem_openapi::{payload::Json, ApiResponse};

use super::ErrorBody;

#[derive(ApiResponse)]
pub enum ApiError {
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 401)]
    Unauthorized(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    Internal(Json<ErrorBody>),
}

impl ApiError {
    pub fn bad_request(message: impl std::fmt::Display) -> Self {
        Self::BadRequest(Json(ErrorBody {
            message: message.to_string(),
        }))
    }

    pub fn unauthorized(required: &str) -> Self {
        Self::Unauthorized(Json(ErrorBody {
            message: format!("a valid {required} is required"),
        }))
    }

    pub fn not_found(what: impl std::fmt::Display) -> Self {
        Self::NotFound(Json(ErrorBody {
            message: format!("{what} not found"),
        }))
    }

    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Internal(Json(ErrorBody {
            message: error.to_string(),
        }))
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
