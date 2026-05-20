use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;
use crate::auth::jwt::verify_token;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;

#[derive(Serialize)]
pub struct EventResponse {
    pub device_id: String,
    pub timestamp: String,
    pub image_url: String,
    pub bucket: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn get_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<ErrorResponse>)> {

    // Verify JWT
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse { error: "Missing token".to_string() }),
        ))?;

    let claims = verify_token(token).map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse { error: "Invalid token".to_string() }),
    ))?;

    let user_id: i64 = claims.sub.parse().unwrap_or(0);

    // Get user's cameras from SQLite
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

    let camera_ids: Vec<String> = rows.into_iter().map(|(id,)| id).collect();

    // Setup DynamoDB client
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let dynamo = Client::new(&config);
    let table = std::env::var("AWS_DYNAMODB_TABLE").unwrap_or("image_events".to_string());

    let mut all_events: Vec<EventResponse> = Vec::new();

    // Fetch events for each camera
    for cam in &camera_ids {
        let result = dynamo
            .query()
            .table_name(&table)
            .key_condition_expression("device_id = :did")
            .expression_attribute_values(
                ":did",
                aws_sdk_dynamodb::types::AttributeValue::S(cam.clone()),
            )
            .limit(50)
            .scan_index_forward(false)
            .send()
            .await
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: e.to_string() }),
            ))?;

        if let Some(items) = result.items {
            for item in items {
                let device_id = item.get("device_id")
                    .and_then(|v| v.as_s().ok())
                    .cloned()
                    .unwrap_or_default();
                let timestamp = item.get("timestamp")
                    .and_then(|v| v.as_s().ok())
                    .cloned()
                    .unwrap_or_default();
                let image_url = item.get("image_url")
                    .and_then(|v| v.as_s().ok())
                    .cloned()
                    .unwrap_or_default();
                let bucket = item.get("bucket")
                    .and_then(|v| v.as_s().ok())
                    .cloned()
                    .unwrap_or_default();

                all_events.push(EventResponse {
                    device_id,
                    timestamp,
                    image_url,
                    bucket,
                });
            }
        }
    }

    // Sort by timestamp descending
    all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    all_events.truncate(50);

    Ok(Json(all_events))
}