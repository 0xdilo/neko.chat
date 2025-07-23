use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;
use dashmap::DashMap;

use crate::error::AppError;
use crate::llm::LLMClient;
use crate::ws_messages::{WsMessage, WsMessageType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingState {
    pub id: String,
    pub message_id: String,
    pub chat_id: String,
    pub user_id: String,
    pub status: StreamingStatus,
    pub content: String,
    pub provider: String,
    pub model: String,
    pub total_tokens: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub last_chunk_index: i32,
    pub chunks: Vec<StreamChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub index: i32,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamingStatus {
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

impl ToString for StreamingStatus {
    fn to_string(&self) -> String {
        match self {
            StreamingStatus::Streaming => "streaming".to_string(),
            StreamingStatus::Completed => "completed".to_string(),
            StreamingStatus::Failed => "failed".to_string(),
            StreamingStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

impl std::str::FromStr for StreamingStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "streaming" => Ok(StreamingStatus::Streaming),
            "completed" => Ok(StreamingStatus::Completed),
            "failed" => Ok(StreamingStatus::Failed),
            "cancelled" => Ok(StreamingStatus::Cancelled),
            _ => Err(format!("Invalid streaming status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingUpdate {
    pub stream_id: String,
    pub message_id: String,
    pub chat_id: String,
    pub user_id: String,
    pub content_delta: String,  // Keep for backwards compatibility
    pub content: String,        // Full accumulated content
    pub chunk_index: i32,
    pub is_complete: bool,
    pub error: Option<String>,
    pub tokens_used: Option<i32>,
}

pub struct ActiveStream {
    pub state: StreamingState,
    pub cancel_tx: mpsc::Sender<()>,
    pub update_tx: broadcast::Sender<StreamingUpdate>,
}

#[derive(Clone)]
pub struct ConnectionInfo {
    pub socket_id: String,
    pub user_id: String,
    pub last_seen: DateTime<Utc>,
}

#[derive(Clone)]
pub struct StreamingManager {
    db_pool: PgPool,
    active_streams: Arc<DashMap<String, ActiveStream>>,
    user_connections: Arc<DashMap<String, Vec<ConnectionInfo>>>,
    ws_broadcast_tx: broadcast::Sender<WsMessage>,
    chunk_cache: Arc<DashMap<String, Vec<StreamChunk>>>,
}

impl StreamingManager {
    pub fn new(db_pool: PgPool, ws_broadcast_tx: broadcast::Sender<WsMessage>) -> Self {
        let manager = Self {
            db_pool,
            active_streams: Arc::new(DashMap::new()),
            user_connections: Arc::new(DashMap::new()),
            ws_broadcast_tx,
            chunk_cache: Arc::new(DashMap::new()),
        };

        // Start connection cleanup task
        let cleanup_manager = manager.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                cleanup_manager.cleanup_stale_connections().await;
            }
        });

        // Start streaming state cleanup task
        let state_cleanup_manager = manager.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;
                let _ = state_cleanup_manager.cleanup_old_streams(24).await;
            }
        });

        manager
    }

    pub async fn register_connection(&self, user_id: String, socket_id: String) {
        let connection = ConnectionInfo {
            socket_id: socket_id.clone(),
            user_id: user_id.clone(),
            last_seen: Utc::now(),
        };

        self.user_connections.entry(user_id.clone())
            .and_modify(|connections| {
                connections.retain(|c| c.socket_id != socket_id);
                connections.push(connection.clone());
            })
            .or_insert_with(|| vec![connection]);

        info!("Registered connection {} for user {}", socket_id, user_id);

        // Check for active streams for this user and send resume messages
        // Small delay to ensure WebSocket task is fully set up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.check_and_resume_active_streams(&user_id).await;
    }

    pub async fn unregister_connection(&self, user_id: &str, socket_id: &str) {
        self.user_connections.entry(user_id.to_string())
            .and_modify(|connections| {
                connections.retain(|c| c.socket_id != socket_id);
            });

        info!("Unregistered connection {} for user {}", socket_id, user_id);
    }

    pub async fn check_and_resume_active_streams(&self, user_id: &str) {
        // Only query for truly active streams
        // Completed streams should not be "resumed" as they're already done
        let active_streams = sqlx::query!(
            "SELECT id, message_id, chat_id, content, provider, model, status,
                    COALESCE(last_chunk_index, -1) as last_chunk_index,
                    COALESCE(chunks, '[]'::jsonb) as chunks
             FROM streaming_states 
             WHERE user_id = $1 AND status = 'streaming'
             ORDER BY created_at ASC",
            user_id
        )
        .fetch_all(&self.db_pool)
        .await;

        if let Ok(streams) = active_streams {
            for stream in streams {
                let stream_id = stream.id;
                let message_id = stream.message_id;
                let chat_id = stream.chat_id;
                let content = stream.content;
                let last_chunk_index = stream.last_chunk_index.unwrap_or(-1);

                info!("Resuming active stream {} for user {} with {} characters: '{}'", 
                    stream_id, user_id, content.len(), 
                    if content.len() > 200 { &content[..200] } else { &content });
                
                info!("Sending resume message to WebSocket broadcast channel for user_id: {}", user_id);

                // Send resume message for active stream
                let resume_message = crate::ws_messages::WsMessage {
                    message_type: crate::ws_messages::WsMessageType::StreamingResume,
                    data: serde_json::json!({
                        "stream_id": stream_id,
                        "message_id": message_id,
                        "chat_id": chat_id,
                        "user_id": user_id,
                        "content": content,
                        "last_chunk_index": last_chunk_index
                    }),
                };

                if let Err(e) = self.ws_broadcast_tx.send(resume_message) {
                    warn!("Failed to send resume message for stream {}: {}", stream_id, e);
                }

                // Also send a start message to ensure UI is in correct state
                let start_message = crate::ws_messages::WsMessage {
                    message_type: crate::ws_messages::WsMessageType::StreamingStart,
                    data: serde_json::json!({
                        "stream_id": stream_id,
                        "message_id": message_id,
                        "chat_id": chat_id,
                        "user_id": user_id
                    }),
                };

                if let Err(e) = self.ws_broadcast_tx.send(start_message) {
                    warn!("Failed to send start message for stream {}: {}", stream_id, e);
                }
            }
        }
    }

    pub async fn update_connection_heartbeat(&self, user_id: &str, socket_id: &str) {
        self.user_connections.entry(user_id.to_string())
            .and_modify(|connections| {
                if let Some(conn) = connections.iter_mut().find(|c| c.socket_id == socket_id) {
                    conn.last_seen = Utc::now();
                }
            });
    }

    async fn cleanup_stale_connections(&self) {
        let stale_threshold = Utc::now() - chrono::Duration::seconds(90); // 90 seconds timeout
        
        for mut entry in self.user_connections.iter_mut() {
            let user_id = entry.key().clone();
            let connections = entry.value_mut();
            
            let initial_count = connections.len();
            connections.retain(|c| c.last_seen > stale_threshold);
            
            if connections.len() < initial_count {
                info!("Cleaned up {} stale connections for user {}", 
                    initial_count - connections.len(), user_id);
            }
        }
    }

    pub async fn start_stream(
        &self,
        message_id: String,
        chat_id: String,
        user_id: String,
        provider: String,
        model: String,
        llm_client: Box<dyn LLMClient>,
        messages: Vec<serde_json::Value>,
    ) -> Result<String, AppError> {
        let stream_id = Uuid::new_v4().to_string();
        
        // Check if there's already an active stream for this message
        if let Some(existing) = self.get_active_stream_for_message(&message_id).await {
            info!("Found existing stream {} for message {}", existing, message_id);
            return Ok(existing);
        }
        
        let state = StreamingState {
            id: stream_id.clone(),
            message_id: message_id.clone(),
            chat_id: chat_id.clone(),
            user_id: user_id.clone(),
            status: StreamingStatus::Streaming,
            content: String::new(),
            provider: provider.clone(),
            model: model.clone(),
            total_tokens: None,
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            last_chunk_index: -1,
            chunks: Vec::new(),
        };

        // Store in database
        self.save_streaming_state(&state).await?;

        // Create channels for this stream
        let (cancel_tx, cancel_rx) = mpsc::channel(1);
        let (update_tx, _) = broadcast::channel(100);

        let active_stream = ActiveStream {
            state: state.clone(),
            cancel_tx,
            update_tx: update_tx.clone(),
        };

        // Add to active streams
        self.active_streams.insert(stream_id.clone(), active_stream);

        // Start the streaming task
        let manager = self.clone();
        let stream_id_clone = stream_id.clone();
        
        tokio::spawn(async move {
            let result = manager.run_stream(
                stream_id_clone.clone(),
                llm_client,
                messages,
                model,
                cancel_rx,
                update_tx,
            ).await;

            if let Err(e) = result {
                error!("Stream {} failed: {}", stream_id_clone, e);
                let _ = manager.mark_stream_failed(&stream_id_clone, &e.to_string()).await;
            }

            // Remove from active streams
            manager.active_streams.remove(&stream_id_clone);
        });

        Ok(stream_id)
    }

    async fn run_stream(
        &self,
        stream_id: String,
        llm_client: Box<dyn LLMClient>,
        messages: Vec<serde_json::Value>,
        model: String,
        mut cancel_rx: mpsc::Receiver<()>,
        update_tx: broadcast::Sender<StreamingUpdate>,
    ) -> Result<(), AppError> {
        let mut accumulated_content = String::new();
        let mut chunk_index = 0;
        let mut chunks = Vec::new();

        // Get initial state to continue from
        let initial_state = self.get_streaming_state(&stream_id).await?
            .ok_or_else(|| AppError::NotFound)?;

        if initial_state.last_chunk_index >= 0 {
            // Resume from existing state
            accumulated_content = initial_state.content;
            chunk_index = initial_state.last_chunk_index + 1;
            chunks = initial_state.chunks;
            
            info!("Resuming stream {} from chunk {}", stream_id, chunk_index);
        }

        // Create the stream  
        let stream_result = llm_client.chat_stream(&model, messages).await;
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                let error_msg = format!("Failed to create stream: {}", e);
                self.broadcast_stream_update(StreamingUpdate {
                    stream_id: stream_id.clone(),
                    message_id: initial_state.message_id.clone(),
                    chat_id: initial_state.chat_id.clone(),
                    user_id: initial_state.user_id.clone(),
                    content_delta: String::new(),
                    content: accumulated_content.clone(),
                    chunk_index,
                    is_complete: true,
                    error: Some(error_msg.clone()),
                    tokens_used: None,
                }).await;
                return Err(AppError::InternalServerError);
            }
        };

        loop {
            tokio::select! {
                // Check for cancellation
                _ = cancel_rx.recv() => {
                    info!("Stream {} cancelled by user", stream_id);
                    self.mark_stream_cancelled(&stream_id).await?;
                    return Ok(());
                }
                
                // Process stream chunks with timeout
                chunk_result = timeout(Duration::from_secs(30), stream.next()) => {
                    match chunk_result {
                        Ok(Some(Ok(chunk))) => {
                            accumulated_content.push_str(&chunk);
                            
                            let stream_chunk = StreamChunk {
                                index: chunk_index,
                                content: chunk.clone(),
                                timestamp: Utc::now(),
                            };
                            chunks.push(stream_chunk.clone());

                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                message_id: initial_state.message_id.clone(),
                                chat_id: initial_state.chat_id.clone(),
                                user_id: initial_state.user_id.clone(),
                                content_delta: chunk,
                                content: accumulated_content.clone(),  // Send full accumulated content
                                chunk_index,
                                is_complete: false,
                                error: None,
                                tokens_used: None,
                            };

                            // Broadcast to WebSocket
                            self.broadcast_stream_update(update.clone()).await;
                            
                            // Send to stream-specific subscribers
                            let _ = update_tx.send(update);

                            // Update cache
                            self.chunk_cache.insert(stream_id.clone(), chunks.clone());

                            // Update database more frequently for better resume
                            if chunk_index % 3 == 0 {
                                self.update_streaming_content(
                                    &stream_id, 
                                    &accumulated_content, 
                                    chunk_index,
                                    &chunks
                                ).await?;
                            }

                            chunk_index += 1;
                        }
                        Ok(Some(Err(e))) => {
                            let error_msg = format!("Stream error: {}", e);
                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                message_id: initial_state.message_id.clone(),
                                chat_id: initial_state.chat_id.clone(),
                                user_id: initial_state.user_id.clone(),
                                content_delta: String::new(),
                                content: accumulated_content.clone(),
                                chunk_index,
                                is_complete: true,
                                error: Some(error_msg.clone()),
                                tokens_used: None,
                            };
                            
                            self.broadcast_stream_update(update.clone()).await;
                            let _ = update_tx.send(update);
                            
                            self.mark_stream_failed(&stream_id, &error_msg).await?;
                            return Err(AppError::InternalServerError);
                        }
                        Ok(None) => {
                            // Stream completed successfully
                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                message_id: initial_state.message_id.clone(),
                                chat_id: initial_state.chat_id.clone(),
                                user_id: initial_state.user_id.clone(),
                                content_delta: String::new(),
                                content: accumulated_content.clone(),
                                chunk_index,
                                is_complete: true,
                                error: None,
                                tokens_used: None,
                            };
                            
                            self.broadcast_stream_update(update.clone()).await;
                            let _ = update_tx.send(update);

                            // Send explicit completion message via WebSocket
                            let completion_message = crate::ws_messages::WsMessage::new_streaming_complete(
                                stream_id.clone(),
                                initial_state.message_id.clone(),
                                initial_state.chat_id.clone(),
                                initial_state.user_id.clone(),
                            );
                            let _ = self.ws_broadcast_tx.send(completion_message);

                            // Final database update with all content
                            self.update_streaming_content(
                                &stream_id, 
                                &accumulated_content, 
                                chunk_index - 1,
                                &chunks
                            ).await?;
                            
                            self.mark_stream_completed(
                                &stream_id, 
                                &accumulated_content, 
                                chunk_index - 1,
                                &chunks
                            ).await?;
                            
                            // Clear cache on completion
                            self.chunk_cache.remove(&stream_id);
                            
                            return Ok(());
                        }
                        Err(_) => {
                            // Timeout
                            let error_msg = "Stream timeout - no data received for 30 seconds".to_string();
                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                message_id: initial_state.message_id.clone(),
                                chat_id: initial_state.chat_id.clone(),
                                user_id: initial_state.user_id.clone(),
                                content_delta: String::new(),
                                content: accumulated_content.clone(),
                                chunk_index,
                                is_complete: true,
                                error: Some(error_msg.clone()),
                                tokens_used: None,
                            };
                            
                            self.broadcast_stream_update(update.clone()).await;
                            let _ = update_tx.send(update);

                            self.mark_stream_failed(&stream_id, &error_msg).await?;
                            return Err(AppError::InternalServerError);
                        }
                    }
                }
            }
        }
    }

    async fn broadcast_stream_update(&self, update: StreamingUpdate) {
        let ws_message = WsMessage {
            message_type: WsMessageType::StreamingUpdate,
            data: serde_json::to_value(&update).unwrap_or_default(),
        };
        
        let _ = self.ws_broadcast_tx.send(ws_message);
    }

    pub async fn get_stream_resume_data(&self, stream_id: &str, user_id: &str) -> Result<Option<(StreamingState, Vec<StreamChunk>)>, AppError> {
        // First check cache
        if let Some(chunks) = self.chunk_cache.get(stream_id) {
            if let Some(state) = self.get_streaming_state(stream_id).await? {
                if state.user_id == user_id {
                    return Ok(Some((state, chunks.clone())));
                }
            }
        }

        // Fall back to database
        if let Some(state) = self.get_streaming_state(stream_id).await? {
            if state.user_id == user_id {
                return Ok(Some((state.clone(), state.chunks)));
            }
        }

        Ok(None)
    }

    pub async fn cancel_stream(&self, stream_id: &str) -> Result<(), AppError> {
        if let Some(active_stream) = self.active_streams.get(stream_id) {
            let _ = active_stream.cancel_tx.send(()).await;
            info!("Sent cancellation signal to stream {}", stream_id);
        }
        Ok(())
    }

    pub fn subscribe_to_stream(&self, stream_id: &str) -> Option<broadcast::Receiver<StreamingUpdate>> {
        self.active_streams.get(stream_id)
            .map(|stream| stream.update_tx.subscribe())
    }

    async fn get_active_stream_for_message(&self, message_id: &str) -> Option<String> {
        for entry in self.active_streams.iter() {
            if entry.value().state.message_id == message_id {
                return Some(entry.key().clone());
            }
        }
        None
    }

    pub async fn get_streaming_state(&self, stream_id: &str) -> Result<Option<StreamingState>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, message_id, chat_id, user_id, status, content, 
                provider, model, total_tokens, created_at, completed_at, 
                error_message,
                COALESCE(last_chunk_index, -1) as last_chunk_index,
                COALESCE(chunks, '[]'::jsonb) as chunks
            FROM streaming_states 
            WHERE id = $1
            "#
        )
        .bind(stream_id)
        .fetch_optional(&self.db_pool)
        .await?;

        match row {
            Some(row) => {
                let chunks: Vec<StreamChunk> = serde_json::from_value(
                    row.try_get::<serde_json::Value, _>("chunks").unwrap_or_default()
                ).unwrap_or_default();

                Ok(Some(StreamingState {
                    id: row.try_get("id")?,
                    message_id: row.try_get("message_id")?,
                    chat_id: row.try_get("chat_id")?,
                    user_id: row.try_get("user_id")?,
                    status: row.try_get::<String, _>("status")?.parse().unwrap_or(StreamingStatus::Failed),
                    content: row.try_get("content")?,
                    provider: row.try_get("provider")?,
                    model: row.try_get("model")?,
                    total_tokens: row.try_get("total_tokens").ok(),
                    created_at: row.try_get("created_at")?,
                    completed_at: row.try_get("completed_at").ok(),
                    error_message: row.try_get("error_message").ok(),
                    last_chunk_index: row.try_get("last_chunk_index").unwrap_or(-1),
                    chunks,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_user_streaming_states(&self, user_id: &str, status: Option<StreamingStatus>) -> Result<Vec<StreamingState>, AppError> {
        let query_str = if status.is_some() {
            r#"
            SELECT 
                id, message_id, chat_id, user_id, status, content, 
                provider, model, total_tokens, created_at, completed_at, 
                error_message,
                COALESCE(last_chunk_index, -1) as last_chunk_index,
                COALESCE(chunks, '[]'::jsonb) as chunks
            FROM streaming_states 
            WHERE user_id = $1 AND status = $2 
            ORDER BY created_at DESC
            "#
        } else {
            r#"
            SELECT 
                id, message_id, chat_id, user_id, status, content, 
                provider, model, total_tokens, created_at, completed_at, 
                error_message,
                COALESCE(last_chunk_index, -1) as last_chunk_index,
                COALESCE(chunks, '[]'::jsonb) as chunks
            FROM streaming_states 
            WHERE user_id = $1 
            ORDER BY created_at DESC
            "#
        };

        let mut query = sqlx::query(query_str).bind(user_id);
        if let Some(status) = status {
            query = query.bind(status.to_string());
        }

        let rows = query.fetch_all(&self.db_pool).await?;

        let mut states = Vec::new();
        for row in rows {
            let chunks: Vec<StreamChunk> = serde_json::from_value(
                row.try_get::<serde_json::Value, _>("chunks").unwrap_or_default()
            ).unwrap_or_default();

            states.push(StreamingState {
                id: row.try_get("id")?,
                message_id: row.try_get("message_id")?,
                chat_id: row.try_get("chat_id")?,
                user_id: row.try_get("user_id")?,
                status: row.try_get::<String, _>("status")?.parse().unwrap_or(StreamingStatus::Failed),
                content: row.try_get("content")?,
                provider: row.try_get("provider")?,
                model: row.try_get("model")?,
                total_tokens: row.try_get("total_tokens").ok(),
                created_at: row.try_get("created_at")?,
                completed_at: row.try_get("completed_at").ok(),
                error_message: row.try_get("error_message").ok(),
                last_chunk_index: row.try_get("last_chunk_index").unwrap_or(-1),
                chunks,
            });
        }

        Ok(states)
    }

    async fn save_streaming_state(&self, state: &StreamingState) -> Result<(), AppError> {
        let chunks_json = serde_json::to_value(&state.chunks)?;
        
        // First check if the table has the new columns
        let has_new_columns = sqlx::query("SELECT last_chunk_index FROM streaming_states LIMIT 1")
            .fetch_optional(&self.db_pool)
            .await
            .is_ok();
            
        if has_new_columns {
            sqlx::query(
                r#"
                INSERT INTO streaming_states 
                    (id, message_id, chat_id, user_id, status, content, provider, model, 
                     total_tokens, created_at, completed_at, error_message, last_chunk_index, chunks) 
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                "#
            )
            .bind(&state.id)
            .bind(&state.message_id)
            .bind(&state.chat_id)
            .bind(&state.user_id)
            .bind(state.status.to_string())
            .bind(&state.content)
            .bind(&state.provider)
            .bind(&state.model)
            .bind(state.total_tokens)
            .bind(state.created_at)
            .bind(state.completed_at)
            .bind(&state.error_message)
            .bind(state.last_chunk_index)
            .bind(chunks_json)
            .execute(&self.db_pool)
            .await?;
        } else {
            // Fallback to old schema
            sqlx::query(
                r#"
                INSERT INTO streaming_states 
                    (id, message_id, chat_id, user_id, status, content, provider, model, 
                     total_tokens, created_at, completed_at, error_message) 
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#
            )
            .bind(&state.id)
            .bind(&state.message_id)
            .bind(&state.chat_id)
            .bind(&state.user_id)
            .bind(state.status.to_string())
            .bind(&state.content)
            .bind(&state.provider)
            .bind(&state.model)
            .bind(state.total_tokens)
            .bind(state.created_at)
            .bind(state.completed_at)
            .bind(&state.error_message)
            .execute(&self.db_pool)
            .await?;
        }

        Ok(())
    }

    async fn update_streaming_content(
        &self, 
        stream_id: &str, 
        content: &str, 
        last_chunk_index: i32,
        chunks: &[StreamChunk]
    ) -> Result<(), AppError> {
        info!("Updating streaming content for stream {}: {} chars, chunk index: {}", 
            stream_id, content.len(), last_chunk_index);
        let chunks_json = serde_json::to_value(chunks)?;
        
        // Check if new columns exist
        let has_new_columns = sqlx::query("SELECT last_chunk_index FROM streaming_states LIMIT 1")
            .fetch_optional(&self.db_pool)
            .await
            .is_ok();
            
        if has_new_columns {
            sqlx::query(
                r#"
                UPDATE streaming_states 
                SET content = $1, last_chunk_index = $2, chunks = $3 
                WHERE id = $4
                "#
            )
            .bind(content)
            .bind(last_chunk_index)
            .bind(chunks_json)
            .bind(stream_id)
            .execute(&self.db_pool)
            .await?;
        } else {
            // Fallback to old schema
            sqlx::query(
                r#"
                UPDATE streaming_states 
                SET content = $1
                WHERE id = $2
                "#
            )
            .bind(content)
            .bind(stream_id)
            .execute(&self.db_pool)
            .await?;
        }

        Ok(())
    }

    async fn mark_stream_completed(
        &self, 
        stream_id: &str, 
        final_content: &str, 
        last_chunk_index: i32,
        chunks: &[StreamChunk]
    ) -> Result<(), AppError> {
        let chunks_json = serde_json::to_value(chunks)?;
        
        // Check if new columns exist
        let has_new_columns = sqlx::query("SELECT last_chunk_index FROM streaming_states LIMIT 1")
            .fetch_optional(&self.db_pool)
            .await
            .is_ok();
            
        if has_new_columns {
            sqlx::query(
                r#"
                UPDATE streaming_states 
                SET status = 'completed', content = $1, last_chunk_index = $2, 
                    chunks = $3, completed_at = NOW() 
                WHERE id = $4
                "#
            )
            .bind(final_content)
            .bind(last_chunk_index)
            .bind(chunks_json)
            .bind(stream_id)
            .execute(&self.db_pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE streaming_states 
                SET status = 'completed', content = $1, completed_at = NOW() 
                WHERE id = $2
                "#
            )
            .bind(final_content)
            .bind(stream_id)
            .execute(&self.db_pool)
            .await?;
        }

        // Update the actual message with the final content
        if let Ok(Some(state)) = self.get_streaming_state(stream_id).await {
            let _ = sqlx::query(
                "UPDATE messages SET content = $1 WHERE id = $2"
            )
            .bind(final_content)
            .bind(&state.message_id)
            .execute(&self.db_pool)
            .await;
        }

        info!("Stream {} completed successfully", stream_id);
        Ok(())
    }

    async fn mark_stream_failed(&self, stream_id: &str, error_message: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE streaming_states 
            SET status = 'failed', error_message = $1, completed_at = NOW() 
            WHERE id = $2
            "#
        )
        .bind(error_message)
        .bind(stream_id)
        .execute(&self.db_pool)
        .await?;

        warn!("Stream {} failed: {}", stream_id, error_message);
        Ok(())
    }

    async fn mark_stream_cancelled(&self, stream_id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE streaming_states 
            SET status = 'cancelled', completed_at = NOW() 
            WHERE id = $1
            "#
        )
        .bind(stream_id)
        .execute(&self.db_pool)
        .await?;

        info!("Stream {} cancelled", stream_id);
        Ok(())
    }

    pub async fn cleanup_old_streams(&self, older_than_hours: i32) -> Result<u64, AppError> {
        let interval_str = format!("{} hours", older_than_hours);
        let result = sqlx::query(
            r#"
            DELETE FROM streaming_states 
            WHERE created_at < NOW() - INTERVAL $1
            AND status IN ('completed', 'failed', 'cancelled')
            "#
        )
        .bind(interval_str)
        .execute(&self.db_pool)
        .await?;

        info!("Cleaned up {} old streaming states", result.rows_affected());
        Ok(result.rows_affected())
    }
}