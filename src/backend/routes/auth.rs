use axum::{extract::{Query, State}, http::StatusCode, Json, response::Redirect};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::auth::jwt::create_token;
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, RedirectUrl, TokenResponse,
    basic::BasicClient, AuthUrl, TokenUrl, CsrfToken,
};

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

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: String,
    name: String,
}

type AuthError = (StatusCode, Json<ErrorResponse>);

fn internal_error(e: impl ToString) -> AuthError {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
}

fn unauthorized() -> AuthError {
    (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Invalid username or password".to_string() }))
}

fn build_oauth_client() -> BasicClient {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID not set");
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET not set");
    let redirect_url = std::env::var("REDIRECT_URL").expect("REDIRECT_URL not set");

    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
        Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
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
            Ok(Json(LoginResponse { token, user_id: id, username: body.username }))
        }
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {

    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM users WHERE name = ?"
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    if existing.is_some() {
        return Err((StatusCode::CONFLICT, Json(ErrorResponse { error: "Username already taken".to_string() })));
    }

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

    for cam in ["cam01", "cam02", "cam03", "cam04"] {
        sqlx::query("INSERT INTO user_cameras (user_id, camera_id) VALUES (?, ?)")
            .bind(user_id)
            .bind(cam)
            .execute(&state.db)
            .await
            .map_err(internal_error)?;
    }

    let token = create_token(&user_id.to_string(), &body.username);
    Ok(Json(LoginResponse { token, user_id, username: body.username }))
}

pub async fn google_login() -> Redirect {
    let client = build_oauth_client();
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("openid".to_string()))
        .add_scope(oauth2::Scope::new("email".to_string()))
        .add_scope(oauth2::Scope::new("profile".to_string()))
        .url();

    Redirect::to(auth_url.as_str())
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CallbackQuery>,
) -> Result<Redirect, AuthError> {

    let client = build_oauth_client();
    // let http_client = reqwest::Client::new();

    let token = client
    .exchange_code(AuthorizationCode::new(params.code))
    .request_async(oauth2::reqwest::async_http_client)
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    let user_info: GoogleUserInfo = reqwest::Client::new()
    .get("https://www.googleapis.com/oauth2/v1/userinfo")
    .bearer_auth(token.access_token().secret())
    .send()
    .await
    .map_err(|e| internal_error(e.to_string()))?
    .json()
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    let existing = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name FROM users WHERE google_id = ?"
    )
    .bind(&user_info.id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    let (user_id, username) = match existing {
        Some((id, name)) => (id, name),
        None => {
            let result = sqlx::query(
                "INSERT INTO users (google_id, email, name, password_hash) VALUES (?, ?, ?, ?)"
            )
            .bind(&user_info.id)
            .bind(&user_info.email)
            .bind(&user_info.name)
            .bind("")
            .execute(&state.db)
            .await
            .map_err(internal_error)?;

            let user_id = result.last_insert_rowid();

            for cam in ["cam01", "cam02", "cam03", "cam04"] {
                sqlx::query("INSERT INTO user_cameras (user_id, camera_id) VALUES (?, ?)")
                    .bind(user_id)
                    .bind(cam)
                    .execute(&state.db)
                    .await
                    .map_err(internal_error)?;
            }

            (user_id, user_info.name)
        }
    };

    let jwt = create_token(&user_id.to_string(), &username);
    let redirect_url = format!(
        "http://127.0.0.1:8080/app/index.html?token={}&username={}",
        jwt, username
    );

    Ok(Redirect::to(&redirect_url))
}
