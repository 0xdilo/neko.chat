use crate::{auth, database::User, error::AppError, AppState};
use axum::{
    extract::{Query, State},
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RegisterPayload {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct GoogleCallbackQuery {
    code: String,
    state: String,
}

#[derive(Deserialize)]
pub struct GoogleUserInfo {
    id: String,
    email: String,
    name: String,
    picture: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    user: User,
}

pub async fn register(
    State(pool): State<SqlitePool>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<User>, AppError> {
    let password_hash = hash(&payload.password, DEFAULT_COST)?;
    let user_id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_hash)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&user_id)
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(&password_hash)
    .execute(&pool)
    .await?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await?;

    Ok(Json(user))
}

pub async fn login(
    State(app_state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(payload.email)
        .fetch_optional(&app_state.db_pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Check if user has a password (not OAuth user)
    let password_hash = user.password_hash.as_ref().ok_or(AppError::Unauthorized)?;

    if !verify(&payload.password, password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = auth::create_jwt(user.id.clone(), &app_state.config.jwt_secret)?;

    Ok(Json(AuthResponse { token, user }))
}

pub async fn get_me(claims: crate::auth::Claims) -> Result<Json<Value>, AppError> {
    Ok(Json(json!({ "user_id": claims.sub })))
}

pub async fn google_auth_url(State(app_state): State<AppState>) -> Result<Json<Value>, AppError> {
    let client_id = app_state
        .config
        .google_client_id
        .ok_or_else(|| AppError::BadRequest("Google OAuth not configured".to_string()))?;

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/auth?client_id={}&redirect_uri={}&scope=openid email profile&response_type=code&state={}",
        client_id,
        urlencoding::encode(&app_state.config.google_redirect_uri),
        uuid::Uuid::new_v4()
    );

    Ok(Json(json!({ "auth_url": auth_url })))
}

pub async fn google_callback(
    State(app_state): State<AppState>,
    Query(params): Query<GoogleCallbackQuery>,
) -> Result<Json<AuthResponse>, AppError> {
    let client_id = app_state
        .config
        .google_client_id
        .ok_or_else(|| AppError::BadRequest("Google OAuth not configured".to_string()))?;
    let client_secret = app_state
        .config
        .google_client_secret
        .ok_or_else(|| AppError::BadRequest("Google OAuth not configured".to_string()))?;

    // Exchange code for token
    let client = Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", &params.code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &app_state.config.google_redirect_uri),
        ])
        .send()
        .await
        .map_err(|_| AppError::BadRequest("Failed to exchange code for token".to_string()))?;

    let token_data: serde_json::Value = token_response
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Failed to parse token response".to_string()))?;

    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("No access token received".to_string()))?;

    // Get user info from Google
    let user_response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| AppError::BadRequest("Failed to get user info".to_string()))?;

    let google_user: GoogleUserInfo = user_response
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Failed to parse user info".to_string()))?;

    // Check if user exists, create if not
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE google_id = $1 OR email = $2")
        .bind(&google_user.id)
        .bind(&google_user.email)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let user = match user {
        Some(mut existing_user) => {
            // Update Google ID if not set
            if existing_user.google_id.is_none() {
                sqlx::query("UPDATE users SET google_id = $1, avatar_url = $2 WHERE id = $3")
                    .bind(&google_user.id)
                    .bind(&google_user.picture)
                    .bind(&existing_user.id)
                    .execute(&app_state.db_pool)
                    .await?;
                existing_user.google_id = Some(google_user.id);
                existing_user.avatar_url = google_user.picture;
            }
            existing_user
        }
        None => {
            // Create new user
            let user_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO users (id, email, name, google_id, avatar_url, role)
                VALUES ($1, $2, $3, $4, $5, 'user')
                "#,
            )
            .bind(&user_id)
            .bind(&google_user.email)
            .bind(&google_user.name)
            .bind(&google_user.id)
            .bind(&google_user.picture)
            .execute(&app_state.db_pool)
            .await?;

            sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&app_state.db_pool)
                .await?
        }
    };

    let token = auth::create_jwt(user.id.clone(), &app_state.config.jwt_secret)?;

    Ok(Json(AuthResponse { token, user }))
}
