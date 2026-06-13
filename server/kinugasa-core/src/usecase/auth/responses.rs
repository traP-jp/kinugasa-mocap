use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    code: &'static str,
    message: String,
}

impl AuthErrorResponse {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
