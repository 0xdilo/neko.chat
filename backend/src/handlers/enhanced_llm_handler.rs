use crate::{
    auth::Claims,
    database::{Chat, Message},
    error::AppError,
    llm::get_llm_client,
    validation::MessageValidator,
    ws_messages::WsMessage,
    AppState,
};
use async_stream::stream;
use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use futures_util::TryStreamExt;
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::{info, error};

// Helper function to generate chat title from message content
fn generate_chat_title(content: &str) -> String {
    if content.is_empty() {
        return "New Chat".to_string();
    }

    let clean_content = content
        .trim()
        .chars()
        .collect::<String>()
        .replace(char::is_whitespace, " ");

    if clean_content.len() <= 50 {
        return clean_content;
    }

    if let Some(pos) = clean_content.find(&['.', '!', '?'][..]) {
        let sentence = &clean_content[..pos];
        if sentence.len() <= 50 {
            return sentence.trim().to_string();
        }
    }

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
        let truncated: String = clean_content.chars().take(47).collect();
        format!("{}...", truncated)
    } else {
        title
    }
}

#[derive(Deserialize)]
pub struct SendMessagePayload {
    pub content: String,
    pub web_search: Option<bool>,
}

async fn get_decrypted_key(
    pool: &sqlx::PgPool,
    user_id: &str,
    provider: &str,
    encryption_key: &str,
) -> Result<String, AppError> {
    let record: crate::database::UserApiKey =
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
        .map_err(|_| AppError::InternalServerError)
}

async fn prepare_conversation(
    pool: &sqlx::PgPool,
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

    let active_prompts = sqlx::query_as::<_, crate::handlers::settings_handler::SystemPrompt>(
        "SELECT * FROM system_prompts WHERE user_id = (SELECT user_id FROM chats WHERE id = $1) AND is_default = true ORDER BY created_at ASC",
    )
    .bind(&chat.id)
    .fetch_all(pool)
    .await?;

    if !active_prompts.is_empty() {
        let combined = active_prompts
            .iter()
            .map(|p| p.prompt.as_str())
            .collect::<Vec<&str>>()
            .join("\n\n---\n\n");
        conversation.insert(0, json!({ "role": "system", "content": combined }));
    } else if let Some(system_prompt) = &chat.system_prompt {
        conversation.insert(0, json!({ "role": "system", "content": system_prompt }));
    }
    Ok(conversation)
}

async fn validate_chat_access(
    pool: &PgPool,
    chat_id: &str,
    user_id: &str,
) -> Result<(String, String), AppError> {
    let result = sqlx::query_as::<_, (String, String, String)>(
        "SELECT user_id, provider, model FROM chats WHERE id = $1",
    )
    .bind(chat_id)
    .fetch_one(pool)
    .await;

    match result {
        Ok((owner_id, provider, model)) => {
            if owner_id != user_id {
                Err(AppError::Unauthorized)
            } else {
                Ok((provider, model))
            }
        }
        Err(_) => Err(AppError::NotFound),
    }
}

async fn setup_llm_client(
    pool: &PgPool,
    user_id: &str,
    provider: &str,
    encryption_key: &str,
) -> Result<Box<dyn crate::llm::LLMClient>, AppError> {
    let api_key = get_decrypted_key(pool, user_id, provider, encryption_key).await?;
    info!("Using provider: {}", provider);
    get_llm_client(provider, &api_key)
}

async fn insert_user_message(
    pool: &PgPool,
    chat_id: &str,
    content: &str,
) -> Result<Message, AppError> {
    let user_message_id = Uuid::new_v4().to_string();
    
    sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'user', $3) RETURNING *",
    )
    .bind(&user_message_id)
    .bind(chat_id)
    .bind(content)
    .fetch_one(pool)
    .await
    .map_err(AppError::DatabaseError)
}

async fn get_chat_details(pool: &PgPool, chat_id: &str) -> Result<Chat, AppError> {
    sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)
}

pub async fn enhanced_stream_message(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(chat_id): Path<String>,
    Json(payload): Json<SendMessagePayload>,
) -> impl IntoResponse {
    info!("Enhanced stream message called - chat_id: {}, user_id: {}, content: {}", chat_id, claims.sub, payload.content);
    // Validate message content
    if let Err(e) = MessageValidator::validate_content(&payload.content) {
        error!("Message validation failed: {:?}", e);
        return e.into_response();
    }

    let user_id = claims.sub;
    let pool = app_state.db_pool.clone();
    let tx = app_state.tx.clone();
    let encryption_key = app_state.config.encryption_key.clone();
    let streaming_manager = match &app_state.streaming_manager {
        Some(manager) => manager.clone(),
        None => {
            error!("Streaming manager not initialized");
            return AppError::InternalServerError.into_response();
        }
    };

    // Validation and setup
    let (chat_provider, chat_model) = match validate_chat_access(&pool, &chat_id, &user_id).await {
        Ok(info) => info,
        Err(e) => return e.into_response(),
    };

    let llm_client = match setup_llm_client(&pool, &user_id, &chat_provider, &encryption_key).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create LLM client for provider {}: {:?}", &chat_provider, e);
            return e.into_response();
        }
    };
    
    info!("Using provider: {}, model: {}", &chat_provider, &chat_model);

    // Create the response stream
    let response_stream = stream! {
        // Insert user message and get chat details in parallel
        let user_msg_future = insert_user_message(&pool, &chat_id, &payload.content);
        let chat_future = get_chat_details(&pool, &chat_id);

        let (user_msg_result, chat_result) = tokio::join!(user_msg_future, chat_future);

        let user_message = match user_msg_result {
            Ok(m) => m,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        let chat = match chat_result {
            Ok(c) => c,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        // Send user message to websocket
        let _ = tx.send(WsMessage::new_chat_message(user_message.clone()));

        // Prepare conversation
        let conversation = match prepare_conversation(&pool, &chat).await {
            Ok(c) => c,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        // Handle title update asynchronously
        if chat.title == "New Chat" || chat.title.contains("New Chat") {
            let title_pool = pool.clone();
            let title_chat_id = chat_id.clone();
            let title_content = payload.content.clone();
            tokio::spawn(async move {
                let existing_count = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM messages WHERE chat_id = $1 AND role = 'user'",
                )
                .bind(&title_chat_id)
                .fetch_one(&title_pool)
                .await
                .unwrap_or(1);

                if existing_count == 1 {
                    let new_title = generate_chat_title(&title_content);
                    let _ = sqlx::query("UPDATE chats SET title = $1 WHERE id = $2")
                        .bind(&new_title)
                        .bind(&title_chat_id)
                        .execute(&title_pool)
                        .await;
                }
            });
        }

        // Create assistant message placeholder
        let assistant_message_id = Uuid::new_v4().to_string();
        let assistant_message = match sqlx::query_as::<_, Message>(
            "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'assistant', '') RETURNING *",
        )
        .bind(&assistant_message_id)
        .bind(&chat_id)
        .fetch_one(&pool)
        .await {
            Ok(m) => m,
            Err(e) => {
                yield Err(AppError::DatabaseError(e));
                return;
            }
        };

        // Send assistant message placeholder
        let _ = tx.send(WsMessage::new_chat_message(assistant_message.clone()));

        // Start streaming with the streaming manager
        let stream_id = match streaming_manager.start_stream(
            assistant_message_id.clone(),
            chat_id.clone(),
            user_id.clone(),
            chat_provider.clone(),
            chat_model.clone(),
            llm_client,
            conversation,
        ).await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to start stream: {}", e);
                yield Err(e);
                return;
            }
        };

        info!("Started stream {} for message {}", stream_id, assistant_message_id);

        // Send streaming start notification
        let _ = tx.send(WsMessage::new_streaming_start(
            stream_id.clone(),
            assistant_message_id.clone(),
            chat_id.clone(),
            user_id.clone(),
        ));

        // Subscribe to stream updates
        if let Some(mut update_rx) = streaming_manager.subscribe_to_stream(&stream_id) {
            while let Ok(update) = update_rx.recv().await {
                if !update.content_delta.is_empty() {
                    yield Ok(update.content_delta);
                }
                
                if update.is_complete {
                    if let Some(error) = update.error {
                        error!("Stream {} completed with error: {}", stream_id, error);
                        yield Ok(format!("ERROR: {}", error));
                    } else {
                        info!("Stream {} completed successfully", stream_id);
                    }
                    break;
                }
            }
        } else {
            error!("Failed to subscribe to stream {}", stream_id);
            yield Err(AppError::InternalServerError);
        }
    };

    let body_stream = response_stream.map_err(axum::Error::new);
    Response::new(Body::from_stream(body_stream))
}

pub async fn enhanced_regenerate_response(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(chat_id): Path<String>,
) -> impl IntoResponse {
    let user_id = claims.sub;
    let pool = app_state.db_pool.clone();
    let tx = app_state.tx.clone();
    let encryption_key = app_state.config.encryption_key.clone();
    let streaming_manager = match &app_state.streaming_manager {
        Some(manager) => manager.clone(),
        None => {
            error!("Streaming manager not initialized");
            return AppError::InternalServerError.into_response();
        }
    };

    let chat: Chat = match sqlx::query_as("SELECT * FROM chats WHERE id = $1 AND user_id = $2")
        .bind(&chat_id)
        .bind(&user_id)
        .fetch_one(&pool)
        .await
    {
        Ok(c) => c,
        Err(_) => return AppError::NotFound.into_response(),
    };

    let conversation = match prepare_conversation(&pool, &chat).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    let api_key = match get_decrypted_key(&pool, &user_id, &chat.provider, &encryption_key).await {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };

    info!("Using provider: {}, model: {}", &chat.provider, &chat.model);
    let llm_client = match get_llm_client(&chat.provider, &api_key) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create LLM client for provider {}: {:?}", &chat.provider, e);
            return e.into_response();
        }
    };

    // Get the last assistant message to regenerate
    let last_assistant_message: Option<Message> = sqlx::query_as(
        "SELECT * FROM messages WHERE chat_id = $1 AND role = 'assistant' ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&chat_id)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let assistant_message_id = match last_assistant_message {
        Some(msg) => msg.id,
        None => {
            // Create new assistant message if none exists
            let new_id = Uuid::new_v4().to_string();
            match sqlx::query_as::<_, Message>(
                "INSERT INTO messages (id, chat_id, role, content) VALUES ($1, $2, 'assistant', '') RETURNING *",
            )
            .bind(&new_id)
            .bind(&chat_id)
            .fetch_one(&pool)
            .await {
                Ok(m) => {
                    let _ = tx.send(WsMessage::new_chat_message(m.clone()));
                    m.id
                },
                Err(_) => return AppError::InternalServerError.into_response(),
            }
        }
    };

    // Clear the content of the message being regenerated
    let _ = sqlx::query("UPDATE messages SET content = '' WHERE id = $1")
        .bind(&assistant_message_id)
        .execute(&pool)
        .await;

    let response_stream = stream! {
        // Start streaming with the streaming manager
        let stream_id = match streaming_manager.start_stream(
            assistant_message_id.clone(),
            chat_id.clone(),
            user_id.clone(),
            chat.provider.clone(),
            chat.model.clone(),
            llm_client,
            conversation,
        ).await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to start regeneration stream: {}", e);
                yield Err(e);
                return;
            }
        };

        info!("Started regeneration stream {} for message {}", stream_id, assistant_message_id);

        // Send streaming start notification
        let _ = tx.send(WsMessage::new_streaming_start(
            stream_id.clone(),
            assistant_message_id.clone(),
            chat_id.clone(),
            user_id.clone(),
        ));

        // Subscribe to stream updates
        if let Some(mut update_rx) = streaming_manager.subscribe_to_stream(&stream_id) {
            while let Ok(update) = update_rx.recv().await {
                if !update.content_delta.is_empty() {
                    yield Ok(update.content_delta);
                }
                
                if update.is_complete {
                    if let Some(error) = update.error {
                        error!("Regeneration stream {} completed with error: {}", stream_id, error);
                        yield Ok(format!("ERROR: {}", error));
                    } else {
                        info!("Regeneration stream {} completed successfully", stream_id);
                    }
                    break;
                }
            }
        } else {
            error!("Failed to subscribe to regeneration stream {}", stream_id);
            yield Err(AppError::InternalServerError);
        }
    };

    let body_stream = response_stream.map_err(axum::Error::new);
    Response::new(Body::from_stream(body_stream))
}