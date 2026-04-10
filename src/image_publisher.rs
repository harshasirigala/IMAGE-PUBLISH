use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use rumqttc::{AsyncClient, QoS};
use serde::Serialize;
use std::path::Path;
use tokio::time::{sleep, Duration};

#[derive(Serialize)]
struct ImageEvent {
    device_id: String,
    event:     String,
    image_url: String,
    bucket:    String,
    key:       String,
    timestamp: String,
}

pub async fn run_image_publisher(
    client:    AsyncClient,
    device_id: String,
    watch_dir: String,
    bucket:    String,
    topic:     String,
) {
    println!("Watching folder: {}", watch_dir);
    println!("S3 bucket: {}", bucket);
    println!("MQTT topic: {}", topic);

    // ── Setup AWS S3 client ───────────────────────────────────────────────
    let aws_config = aws_config::load_defaults(
        aws_config::BehaviorVersion::latest()
    ).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    println!("Watching for new images...\n");

    let mut last_seen: Option<std::time::SystemTime> = None;

    loop {
        sleep(Duration::from_secs(1)).await;

        let watch_path = Path::new(&watch_dir);

        let entries = match std::fs::read_dir(watch_path) {
            Ok(e)  => e,
            Err(e) => {
                eprintln!("Cannot read watch dir: {}", e);
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext != "jpg" && ext != "jpeg" && ext != "png" {
                continue;
            }

            let modified = match entry.metadata().and_then(|m| m.modified()) {
                Ok(t)  => t,
                Err(_) => continue,
            };

            let is_new = match last_seen {
                None       => true,
                Some(last) => modified > last,
            };

            if is_new {
                last_seen = Some(modified);
                println!("New image detected: {:?}", path);
                sleep(Duration::from_millis(500)).await;

                // Upload and publish — errors handled as strings (Send safe)
                match upload_to_s3(&s3_client, &path, &bucket, &device_id).await {
                    Ok((url, key)) => {
                        println!("Uploaded to S3: {}", url);
                        publish_event(
                            &client, &topic, &device_id, &url, &bucket, &key,
                        ).await;
                    }
                    Err(e) => eprintln!("S3 upload failed: {}", e),
                }
            }
        }
    }
}

// ── Upload — returns String error (Send safe) ─────────────────────────────

async fn upload_to_s3(
    client:    &aws_sdk_s3::Client,
    path:      &Path,
    bucket:    &str,
    device_id: &str,
) -> Result<(String, String), String> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.jpg");

    let timestamp = Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let key       = format!("images/{}/{}-{}", device_id, timestamp, filename);

    let body = ByteStream::from_path(path)
        .await
        .map_err(|e| e.to_string())?;

    client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .content_type("image/jpeg")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let url = format!("https://{}.s3.amazonaws.com/{}", bucket, key);
    Ok((url, key))
}

// ── Publish MQTT event ────────────────────────────────────────────────────

async fn publish_event(
    client:    &AsyncClient,
    topic:     &str,
    device_id: &str,
    image_url: &str,
    bucket:    &str,
    key:       &str,
) {
    let event = ImageEvent {
        device_id: device_id.to_string(),
        event:     "image_captured".to_string(),
        image_url: image_url.to_string(),
        bucket:    bucket.to_string(),
        key:       key.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    let payload = serde_json::to_string(&event).unwrap();
    println!("Publishing MQTT event:");
    println!("   {}", payload);

    match client
        .publish(topic, QoS::AtLeastOnce, false, payload.as_bytes())
        .await
    {
        Ok(_)  => println!("   Published!\n"),
        Err(e) => eprintln!("   Publish failed: {}\n", e),
    }
}