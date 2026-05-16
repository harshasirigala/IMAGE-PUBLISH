use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::auth::jwt::create_token;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: i64,
    pub username: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

type AuthError = (StatusCode, Json<ErrorResponse>);

fn internal_error(e: impl ToString) -> AuthError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: e.to_string() }),
    )
}

fn unauthorized() -> AuthError {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse { error: "Invalid username or password".to_string() }),
    )
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {

    let row = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, password_hash FROM users WHERE name = ?"
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    match row {
        None => Err(unauthorized()),
        Some((id, _name, password_hash)) => {
            if password_hash != body.password {
                return Err(unauthorized());
            }
            let token = create_token(&id.to_string(), &body.username);
            Ok(Json(LoginResponse {
                token,
                user_id: id,
                username: body.username,
            }))
        }
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {

    // Check if user exists
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM users WHERE name = ?"
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse { error: "Username already taken".to_string() }),
        ));
    }

    // Insert user
    let result = sqlx::query(
        "INSERT INTO users (google_id, email, name, password_hash) VALUES (?, ?, ?, ?)"
    )
    .bind(&body.username)
    .bind(&body.username)
    .bind(&body.username)
    .bind(&body.password)
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    let user_id = result.last_insert_rowid();

    // Assign cameras
    for cam in ["cam01", "cam02", "cam03", "cam04"] {
        sqlx::query(
            "INSERT INTO user_cameras (user_id, camera_id) VALUES (?, ?)"
        )
        .bind(user_id)
        .bind(cam)
        .execute(&state.db)
        .await
        .map_err(internal_error)?;
    }

    let token = create_token(&user_id.to_string(), &body.username);

    Ok(Json(LoginResponse {
        token,
        user_id,
        username: body.username,
    }))
}

pub async fn callback() -> &'static str {
    "callback"
}