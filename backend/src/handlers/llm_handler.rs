use crate::{
    auth::Claims,
    database::{Chat, Message, UserApiKey},
    error::AppError,
    llm::get_llm_client,
    AppState,
};
use async_stream::stream;
use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{StreamExt, TryStreamExt};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// Helper function to generate chat title from message content
fn generate_chat_title(content: &str) -> String {
    if content.is_empty() {
        return "New Chat".to_string();
    }
    
    // Clean the content and get first meaningful part
    let clean_content = content.trim().chars().collect::<String>().replace(char::is_whitespace, " ");
    
    // If it's very short, use as-is
    if clean_content.len() <= 50 {
        return clean_content;
    }
    
    // Try to find a good breaking point at sentence end
    if let Some(pos) = clean_content.find(&['.', '!', '?'][..]) {
        let sentence = &clean_content[..pos];
        if sentence.len() <= 50 {
            return sentence.trim().to_string();
        }
    }
    
    // Find a good word boundary within 50 characters
    let words: Vec<&str> = clean_content.split_whitespace().collect();
    let mut title = String::new();
    
    for word in words {
        if title.len() + word.len() + 1 > 50 {
            break;
        }
        if !title.is_empty() {
            title.push(' ');
        }
        title.push_str(word);
    }
    
    if title.is_empty() {
        // Fallback: truncate at 47 chars and add ellipsis
        let truncated: String = clean_content.chars().take(47).collect();
        format!("{}...", truncated)
    } else {
        title
    }
}

#[derive(Deserialize)]
pub struct SendMessagePayload {
    pub content: String,
}

#[derive(Deserialize)]
pub struct ParallelLLMPayload {
    pub content: String,
    pub models: Vec<ParallelModelConfig>,
}

#[derive(Deserialize)]
pub struct ParallelModelConfig {
    pub provider: String,
    pub model: String,
}

// --- helper function to get decrypted api key ---
async fn get_decrypted_key(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    provider: &str,
    encryption_key: &str,
) -> Result<String, AppError> {
    let record: UserApiKey =
        sqlx::query_as("SELECT * FROM user_api_keys WHERE user_id = $1 AND provider = $2")
            .bind(user_id)
            .bind(provider)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest(format!("api key for provider '{}' not found.", provider))
            })?;

    let mc = new_magic_crypt!(encryption_key, 256);
    mc.decrypt_base64_to_string(&record.encrypted_key)
        .map_err(|_| AppError::InternalServerError) // decryption failed
}

// --- helper to prepare conversation history ---
async fn prepare_conversation(
    pool: &sqlx::SqlitePool,
    chat: &Chat,
) -> Result<Vec<serde_json::Value>, AppError> {
    let history = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE chat_id = $1 ORDER BY created_at DESC LIMIT 10",
    )
    .bind(&chat.id)
    .fetch_all(pool)
    .await?;

    let mut conversation: Vec<serde_json::Value> = history
        .into_iter()
        .rev()
        .map(|msg| json!({ "role": msg.role, "content": msg.content }))
        .collect();

    // Always use current active system prompts instead of stored ones
    let active_prompts = sqlx::query_as::<_, crate::handlers::settings_handler::SystemPrompt>(
        "SELECT * FROM system_prompts WHERE user_id = (SELECT user_id FROM chats WHERE id = $1) AND is_default = true ORDER BY created_at ASC",
    )
    .bind(&chat.id)
    .fetch_all(pool)
    .await?;

    if !active_prompts.is_empty() {
        // Combine all active prompts with separator
        let combined = active_prompts
            .iter()
            .map(|p| p.prompt.as_str())
            .collect::<Vec<&str>>()
            .join("\n\n---\n\n");
        conversation.insert(0, json!({ "role": "system", "content": combined }));
    } else if let Some(system_prompt) = &chat.system_prompt {
        // Fallback to stored system prompt if no active prompts
        conversation.insert(0, json!({ "role": "system", "content": system_prompt }));
    }
    Ok(conversation)
}

// --- non-streaming handler (kept for posterity or specific use cases) ---
pub async fn send_message(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<SendMessagePayload>,
) -> Result<Json<Message>, AppError> {
    let user_id = claims.sub;
    let pool = &app_state.db_pool;

    let chat: Chat = sqlx::query_as("SELECT * FROM chats WHERE id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    let user_message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'user', $3) RETURNING *",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&chat_id)
    .bind(&payload.content)
    .fetch_one(pool)
    .await?;

    let _ = app_state.tx.send(user_message.clone());

    // Update chat title if this is the first user message and chat has generic title
    if chat.title == "New Chat" || chat.title.contains("New Chat") {
        // Check if this is the first user message
        let existing_user_messages = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM messages WHERE chat_id = $1 AND role = 'user'",
        )
        .bind(&chat_id)
        .fetch_one(pool)
        .await
        .unwrap_or(1);

        if existing_user_messages == 1 {
            // This is the first user message, update the title
            let new_title = generate_chat_title(&payload.content);
            let _ = sqlx::query("UPDATE chats SET title = $1 WHERE id = $2")
                .bind(&new_title)
                .bind(&chat_id)
                .execute(pool)
                .await;
        }
    }

    let conversation = prepare_conversation(pool, &chat).await?;

    let api_key = get_decrypted_key(
        pool,
        &user_id,
        &chat.provider,
        &app_state.config.encryption_key,
    )
    .await?;

    let llm_client = get_llm_client(&chat.provider, &api_key)?;
    let assistant_content = llm_client.chat(&chat.model, conversation).await?;

    let assistant_message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'assistant', $3) RETURNING *",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&chat_id)
    .bind(assistant_content)
    .fetch_one(pool)
    .await?;

    let _ = app_state.tx.send(assistant_message.clone());

    Ok(Json(assistant_message))
}

// --- streaming handler ---
pub async fn stream_message(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<SendMessagePayload>,
) -> impl IntoResponse {
    let user_id = claims.sub;
    let pool = app_state.db_pool.clone();
    let tx = app_state.tx.clone();
    let encryption_key = app_state.config.encryption_key.clone();

    // --- 1. initial db operations & validation ---
    let chat: Chat = match sqlx::query_as("SELECT * FROM chats WHERE id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .fetch_one(&pool)
        .await
    {
        Ok(c) => c,
        Err(_) => return AppError::NotFound.into_response(),
    };

    let user_message = match sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'user', $3) RETURNING *",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&chat_id)
    .bind(&payload.content)
    .fetch_one(&pool)
    .await
    {
        Ok(m) => m,
        Err(e) => return AppError::DatabaseError(e).into_response(),
    };

    let _ = tx.send(user_message.clone());

    // Update chat title if this is the first user message and chat has generic title
    if chat.title == "New Chat" || chat.title.contains("New Chat") {
        // Check if this is the first user message
        let existing_user_messages = match sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM messages WHERE chat_id = $1 AND role = 'user'",
        )
        .bind(&chat_id)
        .fetch_one(&pool)
        .await
        {
            Ok(count) => count,
            Err(_) => 1, // Assume it's the first if query fails
        };

        if existing_user_messages == 1 {
            // This is the first user message, update the title
            let new_title = generate_chat_title(&payload.content);
            let _ = sqlx::query("UPDATE chats SET title = $1 WHERE id = $2")
                .bind(&new_title)
                .bind(&chat_id)
                .execute(&pool)
                .await;
        }
    }

    // --- 2. prepare for llm call ---
    let conversation = match prepare_conversation(&pool, &chat).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    let api_key = match get_decrypted_key(&pool, &user_id, &chat.provider, &encryption_key).await {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };

    tracing::info!("Using provider: {}, model: {}", &chat.provider, &chat.model);
    let llm_client = match get_llm_client(&chat.provider, &api_key) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create LLM client for provider {}: {:?}", &chat.provider, e);
            return e.into_response();
        }
    };

    // --- 3. create the stream ---
    let response_stream = stream! {
        let full_response = Arc::new(tokio::sync::Mutex::new(String::new()));
        let mut llm_stream = match llm_client.chat_stream(&chat.model, conversation).await {
            Ok(s) => s,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        while let Some(chunk_result) = llm_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    full_response.lock().await.push_str(&chunk);
                    yield Ok(chunk);
                }
                Err(e) => {
                    yield Err(e);
                    break;
                }
            }
        }

        // --- 4. after stream finishes, save the full response ---
        let final_content = full_response.lock().await.clone();
        if !final_content.is_empty() {
            let assistant_message = sqlx::query_as::<_, Message>(
                "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'assistant', $3) RETURNING *",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&chat_id)
            .bind(final_content)
            .fetch_one(&pool)
            .await;

            if let Ok(msg) = assistant_message {
                let _ = tx.send(msg);
            }
        }
    };

    let body_stream = response_stream.map_err(axum::Error::new);
    Response::new(Body::from_stream(body_stream))
}

// --- regenerate response handler (for message editing) ---
pub async fn regenerate_response(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(chat_id): Path<String>,
) -> impl IntoResponse {
    let user_id = claims.sub;
    let pool = app_state.db_pool.clone();
    let tx = app_state.tx.clone();
    let encryption_key = app_state.config.encryption_key.clone();

    // --- 1. initial db operations & validation ---
    let chat: Chat = match sqlx::query_as("SELECT * FROM chats WHERE id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .fetch_one(&pool)
        .await
    {
        Ok(c) => c,
        Err(_) => return AppError::NotFound.into_response(),
    };

    // --- 2. prepare for llm call (don't create a new user message) ---
    let conversation = match prepare_conversation(&pool, &chat).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    let api_key = match get_decrypted_key(&pool, &user_id, &chat.provider, &encryption_key).await {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };

    tracing::info!("Using provider: {}, model: {}", &chat.provider, &chat.model);
    let llm_client = match get_llm_client(&chat.provider, &api_key) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create LLM client for provider {}: {:?}", &chat.provider, e);
            return e.into_response();
        }
    };

    // --- 3. create the stream ---
    let response_stream = stream! {
        let full_response = Arc::new(tokio::sync::Mutex::new(String::new()));
        let mut llm_stream = match llm_client.chat_stream(&chat.model, conversation).await {
            Ok(s) => s,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        while let Some(chunk_result) = llm_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    full_response.lock().await.push_str(&chunk);
                    yield Ok(chunk);
                }
                Err(e) => {
                    yield Err(e);
                    break;
                }
            }
        }

        // --- 4. after stream finishes, save the full response ---
        let final_content = full_response.lock().await.clone();
        if !final_content.is_empty() {
            let assistant_message = sqlx::query_as::<_, Message>(
                "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'assistant', $3) RETURNING *",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&chat_id)
            .bind(final_content)
            .fetch_one(&pool)
            .await;

            if let Ok(msg) = assistant_message {
                let _ = tx.send(msg);
            }
        }
    };

    let body_stream = response_stream.map_err(axum::Error::new);
    Response::new(Body::from_stream(body_stream))
}

// --- parallel llm handler ---
pub async fn parallel_llm_query(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<ParallelLLMPayload>,
) -> Result<Json<Vec<Chat>>, AppError> {
    let user_id = claims.sub;
    let pool = &app_state.db_pool;

    let parent_chat: Chat = sqlx::query_as("SELECT * FROM chats WHERE id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| AppError::NotFound)?;

    // Add user message to parent chat first
    let user_message = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'user', $3) RETURNING *",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&chat_id)
    .bind(&payload.content)
    .fetch_one(pool)
    .await?;

    let _ = app_state.tx.send(user_message.clone());

    // Update parent chat title if it has a generic title and this is the first user message
    if parent_chat.title == "New Chat" || parent_chat.title.contains("New Chat") {
        let existing_user_messages = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM messages WHERE chat_id = $1 AND role = 'user'",
        )
        .bind(&chat_id)
        .fetch_one(pool)
        .await
        .unwrap_or(1);

        if existing_user_messages == 1 {
            let new_title = generate_chat_title(&payload.content);
            let _ = sqlx::query("UPDATE chats SET title = $1 WHERE id = $2")
                .bind(&new_title)
                .bind(&chat_id)
                .execute(pool)
                .await;
        }
    }

    let mut created_chats = Vec::new();

    // Create branch chats for each model
    for model_config in payload.models {
        let branch_chat_id = Uuid::new_v4().to_string();
        
        // Generate a better title for branch chats
        let base_title = if parent_chat.title == "New Chat" || parent_chat.title.contains("New Chat") {
            // Generate title from message content if parent has generic title
            generate_chat_title(&payload.content)
        } else {
            parent_chat.title.clone()
        };
        
        let branch_title = format!("{} ({})", base_title, model_config.model);

        let branch_chat = sqlx::query_as::<_, Chat>(
            r#"
            INSERT INTO chats (id, user_id, title, system_prompt, provider, model, pinned, is_branch, parent_chat_id, branch_point_message_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(&branch_chat_id)
        .bind(&user_id)
        .bind(branch_title)
        .bind(&parent_chat.system_prompt)
        .bind(&model_config.provider)
        .bind(&model_config.model)
        .bind(false) // pinned defaults to false
        .bind(true)  // is_branch is true for branch chats
        .bind(&chat_id)
        .bind(&user_message.id)
        .fetch_one(pool)
        .await?;

        // Copy conversation history to new branch (excluding the new user message we just added)
        let history = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE chat_id = $1 AND id != $2 ORDER BY created_at ASC",
        )
        .bind(&chat_id)
        .bind(&user_message.id)
        .fetch_all(pool)
        .await?;

        for msg in history {
            sqlx::query(
                "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, $3, $4)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&branch_chat_id)
            .bind(&msg.role)
            .bind(&msg.content)
            .execute(pool)
            .await?;
        }

        created_chats.push(branch_chat);
    }

    Ok(Json(created_chats))
}
