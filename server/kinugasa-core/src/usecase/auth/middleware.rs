use super::errors::AuthApiError;
use crate::domain::model::ext_user::{ExternalUserAuthenticator, ExternalUserCredential};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

pub async fn require_external_user<A>(
    State(authenticator): State<A>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthApiError>
where
    A: ExternalUserAuthenticator,
{
    let credential = bearer_token(request.headers().get(header::AUTHORIZATION))?;
    let Some(user) = authenticator.authenticate(credential).await? else {
        return Err(AuthApiError::Unauthorized);
    };

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

fn bearer_token(
    authorization: Option<&header::HeaderValue>,
) -> Result<ExternalUserCredential, AuthApiError> {
    let Some(authorization) = authorization else {
        return Err(AuthApiError::Unauthorized);
    };
    let authorization = authorization
        .to_str()
        .map_err(|_| AuthApiError::InvalidAuthorizationHeader)?;
    let Some(token) = authorization.strip_prefix("Bearer ") else {
        return Err(AuthApiError::InvalidAuthorizationHeader);
    };

    if token.trim().is_empty() {
        return Err(AuthApiError::InvalidAuthorizationHeader);
    }

    Ok(ExternalUserCredential::BearerToken(token.to_owned()))
}
