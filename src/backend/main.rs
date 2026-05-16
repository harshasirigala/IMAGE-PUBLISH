use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use rumqttc::AsyncClient;
use sqlx::sqlite::SqlitePoolOptions;
use std::env;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod auth;
mod db;
mod models;
mod routes;

pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub tx: broadcast::Sender<String>,
    pub mqtt_client: AsyncClient,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    std::fs::create_dir_all("data").expect("Failed to create data directory");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to SQLite");

    db::sqlite::run_migrations(&pool).await;
    info!("Database connected and migrations done");

    // Broadcast channel for MQTT → WebSocket
    let (tx, _rx) = broadcast::channel::<String>(100);

    // Start MQTT listener and get shared client
    let (mqtt_client, _) = routes::mqtt_listener::start_and_get_client(tx.clone()).await;

    let state = Arc::new(AppState {
        db: pool,
        tx: tx.clone(),
        mqtt_client,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health",           get(routes::health::health_check))
        .route("/auth/login",       post(routes::auth::login))
        .route("/auth/register",    post(routes::auth::register))
        .route("/auth/callback",    get(routes::auth::callback))
        .route("/api/cameras",      get(routes::cameras::get_cameras))
        .route("/api/events",       get(routes::events::get_events))
        .route("/api/images/:cam",  get(routes::images::get_images))
        .route("/api/command/:cam", post(routes::commands::send_command))
        .route("/ws",               get(routes::ws::ws_handler))
        .layer(cors)
        .with_state(state);

    let host = env::var("SERVER_HOST").unwrap_or("127.0.0.1".to_string());
    let port = env::var("SERVER_PORT").unwrap_or("8080".to_string());
    let addr = format!("{}:{}", host, port);

    info!("Backend server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}