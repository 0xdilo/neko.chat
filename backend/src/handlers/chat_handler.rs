use crate::{
    auth::Claims,
    database::{Chat, Message},
    error::AppError,
    handlers::settings_handler::SystemPrompt,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateChatPayload {
    title: String,
    system_prompt: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    is_branch: Option<bool>,
    parent_chat_id: Option<String>,
    branch_point_message_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateChatPayload {
    title: Option<String>,
    system_prompt: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    pinned: Option<bool>,
}

#[derive(Deserialize)]
pub struct BulkMessagePayload {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct BulkMessagesPayload {
    pub messages: Vec<BulkMessagePayload>,
}

#[derive(Deserialize)]
pub struct UpdateMessagePayload {
    content: String,
}

pub async fn create_chat(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Json(payload): Json<CreateChatPayload>,
) -> Result<Json<Chat>, AppError> {
    let chat_id = Uuid::new_v4().to_string();
    let user_id = claims.sub;

    // Get combined system prompt if none provided
    let final_system_prompt = match payload.system_prompt {
        Some(prompt) => Some(prompt),
        None => {
            // Get all active system prompts and combine them
            let active_prompts = sqlx::query_as::<_, SystemPrompt>(
                "SELECT * FROM system_prompts WHERE user_id = $1 AND is_default = true ORDER BY created_at ASC",
            )
            .bind(&user_id)
            .fetch_all(&pool)
            .await?;

            if active_prompts.is_empty() {
                None
            } else {
                // Combine all active prompts with separator
                let combined = active_prompts
                    .iter()
                    .map(|p| p.prompt.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n\n---\n\n");
                Some(combined)
            }
        }
    };

    let chat = sqlx::query_as::<_, Chat>(
        r#"
        INSERT INTO chats (id, user_id, title, system_prompt, provider, model, pinned, is_branch, parent_chat_id, branch_point_message_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(chat_id)
    .bind(user_id)
    .bind(payload.title)
    .bind(final_system_prompt)
    .bind(payload.provider.unwrap_or_else(|| "openai".to_string()))
    .bind(payload.model.unwrap_or_else(|| "gpt-4o".to_string()))
    .bind(false) // pinned defaults to false
    .bind(payload.is_branch.unwrap_or(false))
    .bind(payload.parent_chat_id)
    .bind(payload.branch_point_message_id)
    .fetch_one(&pool)
    .await?;

    Ok(Json(chat))
}

pub async fn list_chats(
    State(pool): State<SqlitePool>,
    claims: Claims,
) -> Result<Json<Vec<Chat>>, AppError> {
    let user_id = claims.sub;
    let chats = sqlx::query_as::<_, Chat>(
        "SELECT * FROM chats WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;
    Ok(Json(chats))
}

pub async fn get_messages(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path(chat_id): Path<String>,
) -> Result<Json<Vec<Message>>, AppError> {
    let user_id = claims.sub;
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    let messages = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE chat_id = $1 ORDER BY created_at ASC",
    )
    .bind(chat_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(messages))
}

pub async fn delete_chat(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path(chat_id): Path<String>,
) -> Result<Json<()>, AppError> {
    let user_id = claims.sub;

    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    // Get all child chats (branches) that reference this chat as parent
    let child_chats: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM chats WHERE parent_chat_id = $1 AND user_id = $2"
    )
    .bind(&chat_id)
    .bind(&user_id)
    .fetch_all(&pool)
    .await?;

    // Recursively delete all child chats first
    for (child_id,) in child_chats {
        // Delete messages for child chat
        sqlx::query("DELETE FROM messages WHERE chat_id = $1")
            .bind(&child_id)
            .execute(&pool)
            .await?;
        
        // Delete child chat
        sqlx::query("DELETE FROM chats WHERE id = $1")
            .bind(&child_id)
            .execute(&pool)
            .await?;
    }

    // Delete messages for the main chat
    sqlx::query("DELETE FROM messages WHERE chat_id = $1")
        .bind(&chat_id)
        .execute(&pool)
        .await?;

    // Delete the main chat
    sqlx::query("DELETE FROM chats WHERE id = $1")
        .bind(&chat_id)
        .execute(&pool)
        .await?;

    Ok(Json(()))
}

pub async fn update_chat(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<UpdateChatPayload>,
) -> Result<Json<Chat>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    // Build update query dynamically based on provided fields
    let mut query_parts = Vec::new();
    let mut params: Vec<String> = Vec::new();
    let mut param_index = 1;

    if let Some(title) = payload.title {
        query_parts.push(format!("title = ${}", param_index));
        params.push(title);
        param_index += 1;
    }

    if let Some(system_prompt) = payload.system_prompt {
        query_parts.push(format!("system_prompt = ${}", param_index));
        params.push(system_prompt);
        param_index += 1;
    }

    if let Some(provider) = payload.provider {
        query_parts.push(format!("provider = ${}", param_index));
        params.push(provider);
        param_index += 1;
    }

    if let Some(model) = payload.model {
        query_parts.push(format!("model = ${}", param_index));
        params.push(model);
        param_index += 1;
    }

    if let Some(pinned) = payload.pinned {
        query_parts.push(format!("pinned = ${}", param_index));
        params.push(if pinned { "1" } else { "0" }.to_string());
        param_index += 1;
    }

    if query_parts.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let query = format!(
        "UPDATE chats SET {} WHERE id = ${} RETURNING *",
        query_parts.join(", "),
        param_index
    );
    params.push(chat_id);

    let mut query_builder = sqlx::query_as::<_, Chat>(&query);
    for param in params {
        query_builder = query_builder.bind(param);
    }

    let updated_chat = query_builder.fetch_one(&pool).await?;

    Ok(Json(updated_chat))
}

pub async fn bulk_insert_messages(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<BulkMessagesPayload>,
) -> Result<Json<Vec<Message>>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    let mut inserted_messages = Vec::new();

    // Insert messages one by one to maintain order and get proper IDs
    for message_payload in payload.messages {
        let message_id = Uuid::new_v4().to_string();

        let message = sqlx::query_as::<_, Message>(
            "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(message_id)
        .bind(&chat_id)
        .bind(message_payload.role)
        .bind(message_payload.content)
        .fetch_one(&pool)
        .await?;

        inserted_messages.push(message);
    }

    Ok(Json(inserted_messages))
}

pub async fn update_message(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMessagePayload>,
) -> Result<Json<Message>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    // Update the message content
    let updated_message = sqlx::query_as::<_, Message>(
        "UPDATE messages SET content = $1 WHERE id = $2 AND chat_id = $3 RETURNING *",
    )
    .bind(payload.content)
    .bind(&message_id)
    .bind(&chat_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| AppError::NotFound)?;

    Ok(Json(updated_message))
}

pub async fn delete_message_and_subsequent(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
) -> Result<Json<Vec<String>>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    // Get the message being deleted to find its timestamp
    let target_message: (String,) = sqlx::query_as(
        "SELECT created_at FROM messages WHERE id = $1 AND chat_id = $2"
    )
    .bind(&message_id)
    .bind(&chat_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| AppError::NotFound)?;

    // Get all messages from this timestamp onwards (including the target message)
    let messages_to_delete: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM messages WHERE chat_id = $1 AND created_at >= $2 ORDER BY created_at ASC"
    )
    .bind(&chat_id)
    .bind(&target_message.0)
    .fetch_all(&pool)
    .await?;

    let deleted_ids: Vec<String> = messages_to_delete.iter().map(|(id,)| id.clone()).collect();

    // Delete all messages from the target timestamp onwards
    sqlx::query(
        "DELETE FROM messages WHERE chat_id = $1 AND created_at >= $2"
    )
    .bind(&chat_id)
    .bind(&target_message.0)
    .execute(&pool)
    .await?;

    Ok(Json(deleted_ids))
}

pub async fn delete_subsequent_messages(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
) -> Result<Json<Vec<String>>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    // Get the message timestamp
    let target_message: (String,) = sqlx::query_as(
        "SELECT created_at FROM messages WHERE id = $1 AND chat_id = $2"
    )
    .bind(&message_id)
    .bind(&chat_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| AppError::NotFound)?;

    // Get all messages AFTER this timestamp (excluding the target message)
    let messages_to_delete: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM messages WHERE chat_id = $1 AND created_at > $2 ORDER BY created_at ASC"
    )
    .bind(&chat_id)
    .bind(&target_message.0)
    .fetch_all(&pool)
    .await?;

    let deleted_ids: Vec<String> = messages_to_delete.iter().map(|(id,)| id.clone()).collect();

    // Delete all messages AFTER the target timestamp (not including the target message)
    sqlx::query(
        "DELETE FROM messages WHERE chat_id = $1 AND created_at > $2"
    )
    .bind(&chat_id)
    .bind(&target_message.0)
    .execute(&pool)
    .await?;

    Ok(Json(deleted_ids))
}

pub async fn delete_single_message(
    State(pool): State<SqlitePool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
) -> Result<Json<()>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    let chat_owner: (String,) = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    if chat_owner.0 != user_id {
        return Err(AppError::Unauthorized);
    }

    // Verify the message exists in this chat
    let message_exists: Result<(String,), sqlx::Error> = sqlx::query_as(
        "SELECT id FROM messages WHERE id = $1 AND chat_id = $2"
    )
    .bind(&message_id)
    .bind(&chat_id)
    .fetch_one(&pool)
    .await;

    if message_exists.is_err() {
        return Err(AppError::NotFound);
    }

    // Delete the specific message
    sqlx::query("DELETE FROM messages WHERE id = $1 AND chat_id = $2")
        .bind(&message_id)
        .bind(&chat_id)
        .execute(&pool)
        .await?;

    Ok(Json(()))
}
