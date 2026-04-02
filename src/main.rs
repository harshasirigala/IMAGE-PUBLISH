use chrono::Utc;
use rand::Rng;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, Transport, TlsConfiguration};
use serde::Serialize;
use std::{fs, time::Duration};
use tokio::time;

// ── Sensor Data ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SensorData {
    device_id:   String,
    temperature: f64,
    oxygen:      f64,
    ph:          f64,
    timestamp:   String,
}

impl SensorData {
    fn generate(device_id: &str) -> Self {
        let mut rng = rand::thread_rng();
        SensorData {
            device_id:   device_id.to_string(),
            temperature: round(rng.gen_range(10.0..40.0)),
            oxygen:      round(rng.gen_range(5.0..10.0)),
            ph:          round(rng.gen_range(6.0..8.5)),
            timestamp:   Utc::now().to_rfc3339(),
        }
    }
}

fn round(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

// ── TLS Config — reads raw cert bytes, rumqttc handles the rest ────────────

fn build_tls_config() -> TlsConfiguration {
    let ca   = fs::read("certs/ca.pem")
        .expect("❌ Missing certs/ca.pem");
    let cert = fs::read("certs/device.crt.pem")
        .expect("❌ Missing certs/device.crt.pem");
    let key  = fs::read("certs/device.key.pem")
        .expect("❌ Missing certs/device.key.pem");

    // Simple variant — pass raw PEM bytes, rumqttc parses them internally
    TlsConfiguration::Simple {
        ca,
        alpn:        None,
        client_auth: Some((cert, key)),
    }
}

// ── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let mode          = std::env::var("MODE")
        .unwrap_or_else(|_| "mock".to_string());
    let device_id     = std::env::var("DEVICE_ID")
        .unwrap_or_else(|_| "cam01".to_string());
    let interval_secs = std::env::var("INTERVAL_SECS")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u64>().unwrap_or(5);
    let aws_endpoint  = std::env::var("AWS_IOT_ENDPOINT")
        .unwrap_or_default();

    let topic = format!("sensors/{}/data", device_id);

    let (host, port) = match mode.as_str() {
        "aws" => (aws_endpoint.clone(), 8883u16),
        _     => ("test.mosquitto.org".to_string(), 1883u16),
    };

    println!("╔══════════════════════════════════════╗");
    println!("║       MQTT Sensor Publisher          ║");
    println!("╠══════════════════════════════════════╣");
    println!("║  Mode    : {:26} ║", mode);
    println!("║  Device  : {:26} ║", device_id);
    println!("║  Broker  : {:26} ║", host);
    println!("║  Topic   : {:26} ║", topic);
    println!("║  Interval: {:23}s  ║", interval_secs);
    println!("╚══════════════════════════════════════╝\n");

    // ── MQTT Options ───────────────────────────────────────────────────────
    let client_id = format!("{}-{}", device_id, timestamp());
    let mut opts  = MqttOptions::new(client_id, &host, port);
    opts.set_keep_alive(Duration::from_secs(30));

    // TLS only in aws mode
    if mode == "aws" {
        opts.set_transport(Transport::tls_with_config(build_tls_config()));
    }

    let (client, mut eventloop) = AsyncClient::new(opts, 20);

    // ── Publisher Task ─────────────────────────────────────────────────────
    let client_clone = client.clone();
    let topic_clone  = topic.clone();
    let device_clone = device_id.clone();

    tokio::spawn(async move {
        // Wait for connection
        time::sleep(Duration::from_secs(2)).await;

        let mut interval = time::interval(Duration::from_secs(interval_secs));
        let mut count    = 1u32;

        loop {
            interval.tick().await;

            let data    = SensorData::generate(&device_clone);
            let payload = serde_json::to_string(&data).unwrap();

            println!("┌─ Message #{}", count);
            println!("│  {}", payload);

            match client_clone
                .publish(&topic_clone, QoS::AtLeastOnce, false, payload.as_bytes())
                .await
            {
                Ok(_)  => println!("└─ ✅ Sent\n"),
                Err(e) => println!("└─ ❌ Failed: {}\n", e),
            }

            count += 1;
        }
    });

    // ── Event Loop ─────────────────────────────────────────────────────────
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                println!("🟢 Connected to broker!\n");
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("⚠️  Connection lost: {}. Retrying in 5s...", e);
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