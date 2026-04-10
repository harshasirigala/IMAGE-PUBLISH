mod image_publisher;

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, Transport, TlsConfiguration};
use std::{fs, time::Duration};
use tokio::time;

fn build_tls_config() -> TlsConfiguration {
    let ca   = fs::read("certs/ca.pem")        .expect("Missing certs/ca.pem");
    let cert = fs::read("certs/device.crt.pem").expect("Missing certs/device.crt.pem");
    let key  = fs::read("certs/device.key.pem").expect("Missing certs/device.key.pem");
    TlsConfiguration::Simple { ca, alpn: None, client_auth: Some((cert, key)) }
}

#[tokio::main]
async fn main() {
    let mode      = std::env::var("MODE")
        .unwrap_or_else(|_| "mock".to_string());
    let device_id = std::env::var("DEVICE_ID")
        .unwrap_or_else(|_| "cam01".to_string());
    let aws_endpoint = std::env::var("AWS_IOT_ENDPOINT")
        .unwrap_or_default();
    let s3_bucket = std::env::var("S3_BUCKET")
        .unwrap_or_else(|_| "cam01-images".to_string());
    let watch_dir = std::env::var("WATCH_DIR")
        .unwrap_or_else(|_| "/app/snapshots".to_string());

    let image_topic = format!("images/{}/events", device_id);

    let (host, port) = match mode.as_str() {
        "aws" => (aws_endpoint.clone(), 8883u16),
        _     => ("test.mosquitto.org".to_string(), 1883u16),
    };

    println!("  Mode    : {:26} ", mode);
    println!("  Device  : {:26} ", device_id);
    println!("  Broker  : {:26} ", host);
    println!("  S3      : {:26} ", s3_bucket);
    println!("  Watching: {:26} ", watch_dir);

    // ── MQTT setup ────────────────────────────────────────────────────────
    let client_id = format!("{}-{}", device_id, timestamp());
    let mut opts  = MqttOptions::new(client_id, &host, port);
    opts.set_keep_alive(Duration::from_secs(30));

    if mode == "aws" {
        opts.set_transport(Transport::tls_with_config(build_tls_config()));
    }

    let (client, mut eventloop) = AsyncClient::new(opts, 20);

    // ── Image watcher + S3 uploader ───────────────────────────────────────
    let client_clone = client.clone();
    let device_clone = device_id.clone();

    tokio::spawn(async move {
        time::sleep(Duration::from_secs(2)).await;
        image_publisher::run_image_publisher(
            client_clone,
            device_clone,
            watch_dir,
            s3_bucket,
            image_topic,
        ).await;
    });

    // ── Event loop ────────────────────────────────────────────────────────
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                println!("Connected to AWS IoT Core!\n");
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Connection lost: {}. Retrying in 5s...", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn timestamp() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}