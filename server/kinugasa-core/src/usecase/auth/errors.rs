use super::responses::AuthErrorResponse;
use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum AuthApiError {
    Internal(anyhow::Error),
    InvalidAuthorizationHeader,
    Unauthorized,
}

impl From<anyhow::Error> for AuthApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Internal(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse::new(
                    "internal_server_error",
                    error.to_string(),
                )),
            )
                .into_response(),
            Self::InvalidAuthorizationHeader => (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Bearer")],
                Json(AuthErrorResponse::new(
                    "invalid_authorization_header",
                    "Authorization header must be a Bearer token",
                )),
            )
                .into_response(),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Bearer")],
                Json(AuthErrorResponse::new(
                    "unauthorized",
                    "authentication is required",
                )),
            )
                .into_response(),
        }
    }
}
