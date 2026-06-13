use super::responses::ErrorResponse;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum MocapTeamApiError {
    Conflict(String),
    Forbidden(String),
    Internal(anyhow::Error),
    NotFound(String),
    Validation(String),
}

impl From<anyhow::Error> for MocapTeamApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl IntoResponse for MocapTeamApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Conflict(message) => (
                StatusCode::CONFLICT,
                Json(ErrorResponse::new("conflict", message)),
            )
                .into_response(),
            Self::Forbidden(message) => (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("forbidden", message)),
            )
                .into_response(),
            Self::Internal(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "internal_server_error",
                    error.to_string(),
                )),
            )
                .into_response(),
            Self::NotFound(message) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("not_found", message)),
            )
                .into_response(),
            Self::Validation(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new("validation_error", message)),
            )
                .into_response(),
        }
    }
}
