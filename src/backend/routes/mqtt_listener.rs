use rumqttc::{AsyncClient, EventLoop, MqttOptions, Transport, TlsConfiguration, QoS, Event, Packet};
use std::time::Duration;
use std::fs;
use tokio::sync::broadcast;

fn build_opts() -> MqttOptions {
    let endpoint = std::env::var("AWS_IOT_ENDPOINT").unwrap_or_default();
    let client_id = format!("backend-listener-{}", uuid::Uuid::new_v4());
    let mut opts = MqttOptions::new(client_id, &endpoint, 8883);
    opts.set_keep_alive(Duration::from_secs(30));

    let ca   = fs::read("certs/ca.pem").unwrap_or_default();
    let cert = fs::read("certs/device.crt.pem").unwrap_or_default();
    let key  = fs::read("certs/device.key.pem").unwrap_or_default();

    let tls = TlsConfiguration::Simple {
        ca,
        alpn: None,
        client_auth: Some((cert, key)),
    };
    opts.set_transport(Transport::tls_with_config(tls));
    opts
}

pub async fn start_and_get_client(tx: broadcast::Sender<String>) -> (AsyncClient, broadcast::Sender<String>) {
    let (client, mut eventloop) = AsyncClient::new(build_opts(), 20);

    // Wait for connection before returning
    tokio::spawn(async move {
        run_loop(&mut eventloop, tx).await;
    });

    (client, broadcast::channel::<String>(1).0)
}

pub async fn run(tx: broadcast::Sender<String>) {
    let (_client, mut eventloop) = AsyncClient::new(build_opts(), 20);
    run_loop(&mut eventloop, tx).await;
}

async fn run_loop(eventloop: &mut EventLoop, tx: broadcast::Sender<String>) {
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                println!("MQTT listener subscribed to images/+/events");
            }
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                if let Ok(payload) = std::str::from_utf8(&msg.payload) {
                    println!("MQTT event received: {}", payload);
                    let _ = tx.send(payload.to_string());
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("MQTT listener error: {}. Reconnecting in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}