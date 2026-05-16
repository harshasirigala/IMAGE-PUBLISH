FROM rust:latest as builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
RUN mkdir -p src/backend/routes src/backend/models src/backend/db src/backend/auth && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/image_publisher.rs && \
    echo "pub mod sqlite;" > src/backend/db/mod.rs && \
    echo "use sqlx::SqlitePool; pub async fn run_migrations(_pool: &SqlitePool) {}" > src/backend/db/sqlite.rs && \
    echo "pub mod jwt;" > src/backend/auth/mod.rs && \
    echo "pub fn create_token(_: &str, _: &str) -> String { String::new() } pub fn verify_token(_: &str) -> Result<crate::auth::jwt::Claims, jsonwebtoken::errors::Error> { unimplemented!() } #[derive(Debug, serde::Serialize, serde::Deserialize)] pub struct Claims { pub sub: String, pub email: String, pub exp: usize }" > src/backend/auth/jwt.rs && \
    echo "pub mod auth; pub mod cameras; pub mod commands; pub mod events; pub mod health; pub mod images; pub mod mqtt_listener; pub mod ws;" > src/backend/routes/mod.rs && \
    echo "pub async fn health_check() -> &'static str { \"OK\" }" > src/backend/routes/health.rs && \
    echo "pub async fn login() -> &'static str { \"\" } pub async fn register() -> &'static str { \"\" } pub async fn callback() -> &'static str { \"\" }" > src/backend/routes/auth.rs && \
    echo "pub async fn get_cameras() -> &'static str { \"\" }" > src/backend/routes/cameras.rs && \
    echo "pub async fn get_events() -> &'static str { \"\" }" > src/backend/routes/events.rs && \
    echo "use axum::extract::Path; pub async fn get_images(Path(_cam): Path<String>) -> &'static str { \"\" }" > src/backend/routes/images.rs && \
    echo "use axum::extract::Path; pub async fn send_command(Path(_cam): Path<String>) -> &'static str { \"\" }" > src/backend/routes/commands.rs && \
    echo "pub async fn ws_handler() -> &'static str { \"\" }" > src/backend/routes/ws.rs && \
    echo "use tokio::sync::broadcast; pub async fn start(_tx: broadcast::Sender<String>) {}" > src/backend/routes/mqtt_listener.rs && \
    echo "pub mod user; pub mod camera; pub mod event;" > src/backend/models/mod.rs && \
    echo "pub struct User { pub id: i64, pub google_id: String, pub email: String, pub name: String }" > src/backend/models/user.rs && \
    echo "pub struct Camera { pub id: i64, pub user_id: i64, pub camera_id: String }" > src/backend/models/camera.rs && \
    echo "pub struct ImageEvent { pub device_id: String, pub timestamp: String, pub image_url: String, pub bucket: String }" > src/backend/models/event.rs && \
    echo "fn main() {}" > src/backend/main.rs
RUN cargo build --release --bin camera
RUN rm -rf src

COPY src ./src
RUN cargo build --release --bin camera

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/camera .
COPY certs ./certs
RUN mkdir -p /app/snapshots
CMD ["./camera"]