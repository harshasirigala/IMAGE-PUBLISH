use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;
use crate::auth::jwt::verify_token;

#[derive(Serialize)]
pub struct CameraResponse {
    pub camera_id: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn get_cameras(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<CameraResponse>>, (StatusCode, Json<ErrorResponse>)> {

    // Extract JWT from Authorization header
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse { error: "Missing or invalid token".to_string() }),
        ))?;

    // Verify JWT and get user id
    let claims = verify_token(token).map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse { error: "Invalid token".to_string() }),
    ))?;

    let user_id: i64 = claims.sub.parse().unwrap_or(0);

    // Fetch cameras for this user
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT camera_id FROM user_cameras WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: e.to_string() }),
    ))?;

    let cameras = rows
        .into_iter()
        .map(|(camera_id,)| CameraResponse {
            camera_id,
            status: "Running".to_string(),
        })
        .collect();

    Ok(Json(cameras))
}