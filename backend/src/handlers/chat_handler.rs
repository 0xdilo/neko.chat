use crate::{
    auth::Claims,
    database::{Chat, Message},
    error::AppError,
    handlers::settings_handler::SystemPrompt,
    validation::MessageValidator,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

// Helper function to verify chat ownership
async fn verify_chat_ownership(pool: &PgPool, chat_id: &str, user_id: &str) -> Result<(), AppError> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT user_id FROM chats WHERE id = $1 AND user_id = $2")
        .bind(chat_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    if exists.is_some() {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

// Helper function to get chat with ownership verification
async fn get_user_chat(pool: &PgPool, chat_id: &str, user_id: &str) -> Result<Chat, AppError> {
    sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1 AND user_id = $2")
        .bind(chat_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| AppError::NotFound)
}

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

#[derive(Deserialize)]
pub struct CreateForkPayload {
    message_id: String,
    provider: Option<String>,
    model: Option<String>,
    send_message: Option<bool>,
}

pub async fn create_chat(
    State(pool): State<PgPool>,
    claims: Claims,
    Json(payload): Json<CreateChatPayload>,
) -> Result<Json<Chat>, AppError> {
    // Validate input
    MessageValidator::validate_title(&payload.title)?;
    if let Some(ref provider) = payload.provider {
        MessageValidator::validate_provider(provider)?;
    }
    if let Some(ref model) = payload.model {
        MessageValidator::validate_model(model)?;
    }
    if let Some(ref system_prompt) = payload.system_prompt {
        MessageValidator::validate_system_prompt(system_prompt)?;
    }

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
    State(pool): State<PgPool>,
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
    State(pool): State<PgPool>,
    claims: Claims,
    Path(chat_id): Path<String>,
) -> Result<Json<Vec<Message>>, AppError> {
    let user_id = claims.sub;
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

    let messages = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE chat_id = $1 ORDER BY created_at ASC",
    )
    .bind(chat_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(messages))
}

pub async fn delete_chat(
    State(pool): State<PgPool>,
    claims: Claims,
    Path(chat_id): Path<String>,
) -> Result<Json<()>, AppError> {
    let user_id = claims.sub;

    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

    // Since we have ON DELETE CASCADE, we can delete all child chats and their messages efficiently
    // First delete all child chats (and their messages via CASCADE)
    sqlx::query("DELETE FROM chats WHERE parent_chat_id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    // Delete the main chat (messages will be deleted via CASCADE)
    sqlx::query("DELETE FROM chats WHERE id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    Ok(Json(()))
}

pub async fn update_chat(
    State(pool): State<PgPool>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<UpdateChatPayload>,
) -> Result<Json<Chat>, AppError> {
    // Validate input
    if let Some(ref title) = payload.title {
        MessageValidator::validate_title(title)?;
    }
    if let Some(ref system_prompt) = payload.system_prompt {
        MessageValidator::validate_system_prompt(system_prompt)?;
    }
    if let Some(ref provider) = payload.provider {
        MessageValidator::validate_provider(provider)?;
    }
    if let Some(ref model) = payload.model {
        MessageValidator::validate_model(model)?;
    }

    let user_id = claims.sub;

    // Verify the user owns this chat
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

    // Build update query dynamically based on provided fields
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE chats SET ");
    let mut has_updates = false;

    if let Some(title) = payload.title {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("title = ");
        query_builder.push_bind(title);
        has_updates = true;
    }

    if let Some(system_prompt) = payload.system_prompt {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("system_prompt = ");
        query_builder.push_bind(system_prompt);
        has_updates = true;
    }

    if let Some(provider) = payload.provider {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("provider = ");
        query_builder.push_bind(provider);
        has_updates = true;
    }

    if let Some(model) = payload.model {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("model = ");
        query_builder.push_bind(model);
        has_updates = true;
    }

    if let Some(pinned) = payload.pinned {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("pinned = ");
        query_builder.push_bind(pinned); // Use the boolean directly
        has_updates = true;
    }

    if !has_updates {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    query_builder.push(" WHERE id = ");
    query_builder.push_bind(chat_id);
    query_builder.push(" RETURNING *");

    let updated_chat = query_builder
        .build_query_as::<Chat>()
        .fetch_one(&pool)
        .await?;

    Ok(Json(updated_chat))
}

pub async fn bulk_insert_messages(
    State(pool): State<PgPool>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<BulkMessagesPayload>,
) -> Result<Json<Vec<Message>>, AppError> {
    // Validate bulk messages payload
    MessageValidator::validate_bulk_messages(&payload.messages)?;

    let user_id = claims.sub;

    // Verify the user owns this chat
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

    if payload.messages.is_empty() {
        return Ok(Json(vec![]));
    }

    // Use batch insert for better performance
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO messages (id, chat_id, role, content) "
    );

    query_builder.push_values(payload.messages.iter(), |mut b, message_payload| {
        b.push_bind(Uuid::new_v4().to_string())
         .push_bind(&chat_id)
         .push_bind(&message_payload.role)
         .push_bind(&message_payload.content);
    });

    query_builder.push(" RETURNING *");

    let inserted_messages = query_builder
        .build_query_as::<Message>()
        .fetch_all(&pool)
        .await?;

    Ok(Json(inserted_messages))
}

pub async fn update_message(
    State(pool): State<PgPool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMessagePayload>,
) -> Result<Json<Message>, AppError> {
    // Validate message content
    MessageValidator::validate_content(&payload.content)?;

    let user_id = claims.sub;

    // Verify the user owns this chat
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

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
    State(pool): State<PgPool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
) -> Result<Json<Vec<String>>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

    // Get IDs of messages to delete and delete them in one query
    let deleted_ids: Vec<(String,)> = sqlx::query_as(
        r#"DELETE FROM messages 
           WHERE chat_id = $1 
           AND created_at >= (SELECT created_at FROM messages WHERE id = $2 AND chat_id = $1)
           RETURNING id"#
    )
    .bind(&chat_id)
    .bind(&message_id)
    .fetch_all(&pool)
    .await?;

    if deleted_ids.is_empty() {
        return Err(AppError::NotFound);
    }

    let result: Vec<String> = deleted_ids.into_iter().map(|(id,)| id).collect();
    Ok(Json(result))
}

pub async fn delete_subsequent_messages(
    State(pool): State<PgPool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
) -> Result<Json<Vec<String>>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

    // Get IDs of messages to delete and delete them in one query
    let deleted_ids: Vec<(String,)> = sqlx::query_as(
        r#"DELETE FROM messages 
           WHERE chat_id = $1 
           AND created_at > (SELECT created_at FROM messages WHERE id = $2 AND chat_id = $1)
           RETURNING id"#
    )
    .bind(&chat_id)
    .bind(&message_id)
    .fetch_all(&pool)
    .await?;

    let result: Vec<String> = deleted_ids.into_iter().map(|(id,)| id).collect();
    Ok(Json(result))
}

pub async fn delete_single_message(
    State(pool): State<PgPool>,
    claims: Claims,
    Path((chat_id, message_id)): Path<(String, String)>,
) -> Result<Json<()>, AppError> {
    let user_id = claims.sub;

    // Verify the user owns this chat
    verify_chat_ownership(&pool, &chat_id, &user_id).await?;

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

pub async fn create_fork(
    State(pool): State<PgPool>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<CreateForkPayload>,
) -> Result<Json<Chat>, AppError> {
    let user_id = claims.sub;
    
    // Get the parent chat with ownership verification
    let parent_chat: Chat = get_user_chat(&pool, &chat_id, &user_id).await?;
    
    // Get the message we're forking from
    let fork_message: Message = sqlx::query_as("SELECT * FROM messages WHERE id = $1 AND chat_id = $2")
        .bind(&payload.message_id)
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| AppError::NotFound)?;
    
    // Get all messages up to and including the fork point
    let messages: Vec<Message> = sqlx::query_as(
        "SELECT * FROM messages WHERE chat_id = $1 AND created_at <= $2 ORDER BY created_at ASC"
    )
    .bind(&chat_id)
    .bind(&fork_message.created_at)
    .fetch_all(&pool)
    .await?;
    
    // Create new branch chat
    let new_chat_id = Uuid::new_v4().to_string();
    let new_chat = sqlx::query_as::<_, Chat>(
        r#"
        INSERT INTO chats (id, user_id, title, system_prompt, provider, model, pinned, is_branch, parent_chat_id, branch_point_message_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(&new_chat_id)
    .bind(&user_id)
    .bind(&parent_chat.title)
    .bind(&parent_chat.system_prompt)
    .bind(payload.provider.unwrap_or(parent_chat.provider))
    .bind(payload.model.unwrap_or(parent_chat.model))
    .bind(false)
    .bind(true)
    .bind(&chat_id)
    .bind(&payload.message_id)
    .fetch_one(&pool)
    .await?;
    
    // Copy messages to new chat, excluding the fork message if it's a user message and we want to resend it
    let should_resend = payload.send_message.unwrap_or(true) && fork_message.role == "user";
    let messages_to_copy: Vec<_> = if should_resend {
        // Don't copy the fork message itself - we'll send it fresh
        messages.into_iter().filter(|m| m.id != fork_message.id).collect()
    } else {
        messages
    };
    
    // Insert messages into new chat
    for message in messages_to_copy {
        let new_message_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(new_message_id)
        .bind(&new_chat_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(&message.created_at)
        .execute(&pool)
        .await?;
    }
    
    // If we should resend the fork message, add it to the new chat
    if should_resend {
        let new_message_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(new_message_id)
        .bind(&new_chat_id)
        .bind(&fork_message.role)
        .bind(&fork_message.content)
        .bind(chrono::Utc::now())
        .execute(&pool)
        .await?;
    }
    
    Ok(Json(new_chat))
}
