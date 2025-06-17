use crate::{auth::Claims, database::UserApiKey, error::AppError, AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AddKeyPayload {
    provider: String,
    api_key: String,
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    provider: String,
    created_at: String,
}

#[derive(Serialize)]
pub struct DecryptedApiKeyResponse {
    provider: String,
    api_key: String,
    created_at: String,
}

pub async fn add_key(
    State(app_state): State<AppState>,
    claims: Claims,
    Json(payload): Json<AddKeyPayload>,
) -> Result<Json<ApiKeyResponse>, AppError> {
    let user_id = claims.sub;
    let mc = new_magic_crypt!(&app_state.config.encryption_key, 256);
    let encrypted_key = mc.encrypt_str_to_base64(&payload.api_key);

    // Use UPSERT logic for SQLite
    let key_record = sqlx::query_as::<_, UserApiKey>(
        r#"
        INSERT INTO user_api_keys (user_id, provider, encrypted_key)
        VALUES ($1, $2, $3)
        ON CONFLICT(user_id, provider) DO UPDATE SET
            encrypted_key = excluded.encrypted_key,
            created_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(payload.provider)
    .bind(encrypted_key)
    .fetch_one(&app_state.db_pool)
    .await?;

    Ok(Json(ApiKeyResponse {
        provider: key_record.provider,
        created_at: key_record.created_at,
    }))
}

pub async fn list_keys(
    State(pool): State<sqlx::SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    let user_id = claims.sub;
    let keys = sqlx::query_as::<_, UserApiKey>(
        "SELECT user_id, provider, encrypted_key, created_at FROM user_api_keys WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    let response = keys
        .into_iter()
        .map(|k| ApiKeyResponse {
            provider: k.provider,
            created_at: k.created_at,
        })
        .collect();

    Ok(Json(response))
}

pub async fn delete_key(
    State(pool): State<sqlx::SqlitePool>,
    claims: Claims,
    Path(provider): Path<String>,
) -> Result<(), AppError> {
    let user_id = claims.sub;
    let result = sqlx::query("DELETE FROM user_api_keys WHERE user_id = $1 AND provider = $2")
        .bind(user_id)
        .bind(provider)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        Err(AppError::NotFound)
    } else {
        Ok(())
    }
}

pub async fn get_key(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(provider): Path<String>,
) -> Result<Json<DecryptedApiKeyResponse>, AppError> {
    let user_id = claims.sub;
    let key_record = sqlx::query_as::<_, UserApiKey>(
        "SELECT user_id, provider, encrypted_key, created_at FROM user_api_keys WHERE user_id = $1 AND provider = $2",
    )
    .bind(user_id)
    .bind(&provider)
    .fetch_one(&app_state.db_pool)
    .await
    .map_err(|_| AppError::NotFound)?;

    let mc = new_magic_crypt!(&app_state.config.encryption_key, 256);
    let decrypted_key = mc.decrypt_base64_to_string(&key_record.encrypted_key)
        .map_err(|_| AppError::InternalServerError)?;

    Ok(Json(DecryptedApiKeyResponse {
        provider: key_record.provider,
        api_key: decrypted_key,
        created_at: key_record.created_at,
    }))
}
