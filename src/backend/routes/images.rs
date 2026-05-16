use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;
use crate::auth::jwt::verify_token;
use aws_config::BehaviorVersion;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

#[derive(Serialize)]
pub struct ImageResponse {
    pub key: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn get_images(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(cam): Path<String>,
) -> Result<Json<Vec<ImageResponse>>, (StatusCode, Json<ErrorResponse>)> {

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

    // Setup S3 client
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3 = aws_sdk_s3::Client::new(&config);
    let bucket = std::env::var("AWS_S3_BUCKET")
        .unwrap_or("captured-images-013545650538-us-east-1-an".to_string());

    // List objects for this camera
    let prefix = format!("images/{}/", cam);
    let result = s3
        .list_objects_v2()
        .bucket(&bucket)
        .prefix(&prefix)
        .max_keys(20)
        .send()
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        ))?;

    let mut images: Vec<ImageResponse> = Vec::new();

    if let Some(objects) = result.contents {
        for obj in objects {
            if let Some(key) = obj.key {
                // Generate presigned URL valid for 1 hour
                let presigned = s3
                    .get_object()
                    .bucket(&bucket)
                    .key(&key)
                    .presigned(
                        PresigningConfig::expires_in(Duration::from_secs(3600))
                            .unwrap()
                    )
                    .await
                    .map_err(|e| (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse { error: e.to_string() }),
                    ))?;

                images.push(ImageResponse {
                    key: key.clone(),
                    url: presigned.uri().to_string(),
                });
            }
        }
    }

    // Sort latest first
    images.reverse();

    Ok(Json(images))
}