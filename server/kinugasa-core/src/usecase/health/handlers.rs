use super::responses::HealthResponse;
use axum::Json;

pub async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}
