use crate::{auth::Claims, error::AppError, database::UserModel, llm::{fetch_available_models, NormalizedModel}, AppState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;
use magic_crypt::{new_magic_crypt, MagicCryptTrait};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemPrompt {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub prompt: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub category: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateSystemPromptPayload {
    pub name: String,
    pub prompt: String,
    pub description: Option<String>,
    pub is_default: Option<bool>,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSystemPromptPayload {
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub description: Option<String>,
    pub is_default: Option<bool>,
    pub category: Option<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSettings {
    pub user_id: String,
    pub theme: String,
    pub language: String,
    pub font_size: i32,
    pub notifications_enabled: bool,
    pub auto_save: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct UpdateSettingsPayload {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub font_size: Option<i32>,
    pub notifications_enabled: Option<bool>,
    pub auto_save: Option<bool>,
}

pub async fn get_settings(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<UserSettings>, AppError> {
    let user_id = claims.sub;

    let settings =
        sqlx::query_as::<_, UserSettings>("SELECT * FROM user_settings WHERE user_id = $1")
            .bind(&user_id)
            .fetch_optional(&pool)
            .await?;

    let settings = match settings {
        Some(s) => s,
        None => {
            // Create default settings if none exist
            let default_settings = sqlx::query_as::<_, UserSettings>(
                r#"
                INSERT INTO user_settings (user_id, theme, language, font_size, notifications_enabled, auto_save)
                VALUES ($1, 'dark', 'en', 14, true, true)
                RETURNING *
                "#
            )
            .bind(&user_id)
            .fetch_one(&pool)
            .await?;
            default_settings
        }
    };

    Ok(Json(settings))
}

pub async fn update_settings(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Json(payload): Json<UpdateSettingsPayload>,
) -> Result<Json<UserSettings>, AppError> {
    let user_id = claims.sub;

    let settings = sqlx::query_as::<_, UserSettings>(
        r#"
        UPDATE user_settings 
        SET theme = COALESCE($1, theme),
            language = COALESCE($2, language),
            font_size = COALESCE($3, font_size),
            notifications_enabled = COALESCE($4, notifications_enabled),
            auto_save = COALESCE($5, auto_save),
            updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
        WHERE user_id = $6
        RETURNING *
        "#,
    )
    .bind(payload.theme)
    .bind(payload.language)
    .bind(payload.font_size)
    .bind(payload.notifications_enabled)
    .bind(payload.auto_save)
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    Ok(Json(settings))
}

pub async fn get_system_prompts(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<SystemPrompt>>, AppError> {
    let user_id = claims.sub;

    let prompts = sqlx::query_as::<_, SystemPrompt>(
        "SELECT * FROM system_prompts WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(prompts))
}

pub async fn get_active_system_prompts(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<SystemPrompt>>, AppError> {
    let user_id = claims.sub;

    let prompts = sqlx::query_as::<_, SystemPrompt>(
        "SELECT * FROM system_prompts WHERE user_id = $1 AND is_default = true ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(prompts))
}

pub async fn create_system_prompt(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Json(payload): Json<CreateSystemPromptPayload>,
) -> Result<Json<SystemPrompt>, AppError> {
    let user_id = claims.sub;
    let prompt_id = Uuid::new_v4().to_string();

    // Note: We now allow multiple active prompts, so we don't unset existing defaults

    let prompt = sqlx::query_as::<_, SystemPrompt>(
        r#"
        INSERT INTO system_prompts (id, user_id, name, prompt, description, is_default, category)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(prompt_id)
    .bind(&user_id)
    .bind(payload.name)
    .bind(payload.prompt)
    .bind(payload.description)
    .bind(payload.is_default.unwrap_or(false))
    .bind(payload.category.unwrap_or_else(|| "general".to_string()))
    .fetch_one(&pool)
    .await?;

    Ok(Json(prompt))
}

pub async fn update_system_prompt(
    State(pool): State<SqlitePool>,
    claims: Claims,
    axum::extract::Path(prompt_id): axum::extract::Path<String>,
    Json(payload): Json<UpdateSystemPromptPayload>,
) -> Result<Json<SystemPrompt>, AppError> {
    let user_id = claims.sub;

    // Verify ownership
    let owner_check: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM system_prompts WHERE id = $1")
            .bind(&prompt_id)
            .fetch_optional(&pool)
            .await?;

    if let Some((owner_id,)) = owner_check {
        if owner_id != user_id.to_string() {
            return Err(AppError::Unauthorized);
        }
    } else {
        return Err(AppError::NotFound);
    }

    // Note: We now allow multiple active prompts, so we don't unset existing defaults

    let prompt = sqlx::query_as::<_, SystemPrompt>(
        r#"
        UPDATE system_prompts 
        SET name = COALESCE($1, name),
            prompt = COALESCE($2, prompt),
            description = COALESCE($3, description),
            is_default = COALESCE($4, is_default),
            category = COALESCE($5, category)
        WHERE id = $6 AND user_id = $7
        RETURNING *
        "#,
    )
    .bind(payload.name)
    .bind(payload.prompt)
    .bind(payload.description)
    .bind(payload.is_default)
    .bind(payload.category)
    .bind(&prompt_id)
    .bind(&user_id)
    .fetch_one(&pool)
    .await?;

    Ok(Json(prompt))
}

pub async fn delete_system_prompt(
    State(pool): State<SqlitePool>,
    claims: Claims,
    axum::extract::Path(prompt_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = claims.sub;

    let result = sqlx::query("DELETE FROM system_prompts WHERE id = $1 AND user_id = $2")
        .bind(prompt_id)
        .bind(user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn set_active_prompt(
    State(pool): State<SqlitePool>,
    claims: Claims,
    axum::extract::Path(prompt_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = claims.sub;

    // Verify ownership
    let owner_check: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM system_prompts WHERE id = $1")
            .bind(&prompt_id)
            .fetch_optional(&pool)
            .await?;

    if let Some((owner_id,)) = owner_check {
        if owner_id != user_id.to_string() {
            return Err(AppError::Unauthorized);
        }
    } else {
        return Err(AppError::NotFound);
    }

    // Unset all defaults first
    sqlx::query("UPDATE system_prompts SET is_default = false WHERE user_id = $1")
        .bind(&user_id)
        .execute(&pool)
        .await?;

    // Set the selected prompt as default
    sqlx::query("UPDATE system_prompts SET is_default = true WHERE id = $1 AND user_id = $2")
        .bind(&prompt_id)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn toggle_active_prompt(
    State(pool): State<SqlitePool>,
    claims: Claims,
    axum::extract::Path(prompt_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = claims.sub;

    // Verify ownership and get current state
    let prompt_check: Option<(String, bool)> =
        sqlx::query_as("SELECT user_id, is_default FROM system_prompts WHERE id = $1")
            .bind(&prompt_id)
            .fetch_optional(&pool)
            .await?;

    if let Some((owner_id, is_currently_active)) = prompt_check {
        if owner_id != user_id.to_string() {
            return Err(AppError::Unauthorized);
        }
        
        // Toggle the active state
        let new_state = !is_currently_active;
        sqlx::query("UPDATE system_prompts SET is_default = $1 WHERE id = $2 AND user_id = $3")
            .bind(new_state)
            .bind(&prompt_id)
            .bind(&user_id)
            .execute(&pool)
            .await?;

        Ok(Json(serde_json::json!({ 
            "success": true, 
            "is_active": new_state 
        })))
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn get_api_keys(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let user_id = claims.sub;

    let keys = sqlx::query_as::<_, (String, String)>(
        "SELECT provider, created_at FROM user_api_keys WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<serde_json::Value> = keys
        .iter()
        .map(|(provider, created_at)| {
            serde_json::json!({
                "provider": provider,
                "created_at": created_at,
                "has_key": true
            })
        })
        .collect();

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct FetchModelsPayload {
    pub provider: String,
}

#[derive(Deserialize)]
pub struct UpdateModelPreferencesPayload {
    pub models: Vec<UserModelUpdate>,
}

#[derive(Deserialize)]
pub struct UserModelUpdate {
    pub provider: String,
    pub model_id: String,
    pub model_name: String,
    pub is_enabled: bool,
    pub display_order: i32,
}

pub async fn fetch_models_for_provider(
    State(app_state): State<AppState>,
    claims: Claims,
    Json(payload): Json<FetchModelsPayload>,
) -> Result<Json<Vec<NormalizedModel>>, AppError> {
    let user_id = claims.sub;
    let pool = &app_state.db_pool;

    // Get the encrypted API key for this provider
    let api_key_record = sqlx::query_as::<_, (String,)>(
        "SELECT encrypted_key FROM user_api_keys WHERE user_id = $1 AND provider = $2"
    )
    .bind(&user_id)
    .bind(&payload.provider)
    .fetch_optional(pool)
    .await?;

    let encrypted_key = api_key_record
        .ok_or_else(|| AppError::BadRequest(format!("No API key found for provider '{}'", payload.provider)))?
        .0;

    // Decrypt the API key
    let mc = new_magic_crypt!(&app_state.config.encryption_key, 256);
    let api_key = mc.decrypt_base64_to_string(&encrypted_key)
        .map_err(|_| AppError::InternalServerError)?;

    // Fetch models from the provider
    let models = fetch_available_models(&payload.provider, &api_key).await?;

    Ok(Json(models))
}

pub async fn get_user_models(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<UserModel>>, AppError> {
    let user_id = claims.sub;

    let models = sqlx::query_as::<_, UserModel>(
        "SELECT * FROM user_models WHERE user_id = $1 ORDER BY display_order ASC, created_at ASC"
    )
    .bind(&user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(models))
}

pub async fn update_user_model_preferences(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Json(payload): Json<UpdateModelPreferencesPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = claims.sub;

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Clear existing preferences for this user
    sqlx::query("DELETE FROM user_models WHERE user_id = $1")
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;

    // Insert new preferences
    for model in payload.models {
        sqlx::query(
            r#"
            INSERT INTO user_models (user_id, provider, model_id, model_name, is_enabled, display_order)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(&user_id)
        .bind(&model.provider)
        .bind(&model.model_id)
        .bind(&model.model_name)
        .bind(model.is_enabled)
        .bind(model.display_order)
        .execute(&mut *tx)
        .await?;
    }

    // Commit the transaction
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn get_enabled_models_for_user(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<UserModel>>, AppError> {
    let user_id = claims.sub;

    let models = sqlx::query_as::<_, UserModel>(
        "SELECT * FROM user_models WHERE user_id = $1 AND is_enabled = true ORDER BY display_order ASC"
    )
    .bind(&user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(models))
}

#[derive(Deserialize)]
pub struct ToggleModelPayload {
    pub provider: String,
    pub model_id: String,
    pub model_name: String,
}

pub async fn toggle_model_enabled(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Json(payload): Json<ToggleModelPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = claims.sub;

    // Check if model preference exists
    let existing = sqlx::query_as::<_, UserModel>(
        "SELECT * FROM user_models WHERE user_id = $1 AND provider = $2 AND model_id = $3"
    )
    .bind(&user_id)
    .bind(&payload.provider)
    .bind(&payload.model_id)
    .fetch_optional(&pool)
    .await?;

    let is_enabled = if let Some(model) = existing {
        // Toggle existing model
        let new_enabled = !model.is_enabled;
        sqlx::query(
            "UPDATE user_models SET is_enabled = $1 WHERE user_id = $2 AND provider = $3 AND model_id = $4"
        )
        .bind(new_enabled)
        .bind(&user_id)
        .bind(&payload.provider)
        .bind(&payload.model_id)
        .execute(&pool)
        .await?;
        new_enabled
    } else {
        // Create new model preference (enabled by default when toggling)
        sqlx::query(
            "INSERT INTO user_models (user_id, provider, model_id, model_name, is_enabled, display_order) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(&user_id)
        .bind(&payload.provider)
        .bind(&payload.model_id)
        .bind(&payload.model_name)
        .bind(true)
        .bind(0)
        .execute(&pool)
        .await?;
        true
    };

    tracing::info!("Model toggled: {} {} - enabled: {}", payload.provider, payload.model_id, is_enabled);
    
    Ok(Json(serde_json::json!({ 
        "success": true, 
        "is_enabled": is_enabled,
        "model_id": payload.model_id,
        "provider": payload.provider
    })))
}

