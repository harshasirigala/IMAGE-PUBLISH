use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::auth::jwt::verify_token;

#[derive(Deserialize)]
pub struct CommandRequest {
    pub command: String,
}

#[derive(Serialize)]
pub struct CommandResponse {
    pub status: String,
    pub camera: String,
    pub command: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn send_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(cam): Path<String>,
    Json(body): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, (StatusCode, Json<ErrorResponse>)> {

    // Verify JWT
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse { error: "Missing token".to_string() }),
        ))?;

    verify_token(token).map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse { error: "Invalid token".to_string() }),
    ))?;

    // Validate command
    let allowed = ["capture", "status", "ping"];
    if !allowed.contains(&body.command.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: format!("Unknown command: {}", body.command) }),
        ));
    }

    // Trigger webcam capture on host machine
    if body.command == "capture" {
        tokio::process::Command::new("python3")
            .arg("/Users/harshasirigala/mqtt-publisher/capture.py")
            .spawn()
            .ok();
    }

    // Use shared MQTT client from AppState
    let topic = format!("commands/{}", cam);
    let payload = serde_json::json!({
        "command": body.command,
        "camera": cam,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    state.mqtt_client
        .publish(
            &topic,
            rumqttc::QoS::AtLeastOnce,
            false,
            payload.to_string().as_bytes(),
        )
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: format!("MQTT publish failed: {}", e) }),
        ))?;

    Ok(Json(CommandResponse {
        status: "sent".to_string(),
        camera: cam,
        command: body.command,
    }))
}