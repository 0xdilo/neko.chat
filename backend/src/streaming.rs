use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, mpsc};
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::AppError;
use crate::llm::LLMClient;

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

#[derive(Debug, Clone)]
pub struct StreamingUpdate {
    pub stream_id: String,
    pub content_delta: String,
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
pub struct StreamingManager {
    db_pool: PgPool,
    active_streams: Arc<RwLock<HashMap<String, ActiveStream>>>,
    global_update_tx: broadcast::Sender<StreamingUpdate>,
}

impl StreamingManager {
    pub fn new(db_pool: PgPool) -> Self {
        let (global_update_tx, _) = broadcast::channel(1000);
        
        Self {
            db_pool,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            global_update_tx,
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
        let stream_id_for_task = stream_id.clone();
        
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
        {
            let mut streams = self.active_streams.write().await;
            streams.insert(stream_id.clone(), active_stream);
        }

        // Start the streaming task
        let manager = self.clone();
        let update_tx_clone = update_tx.clone();
        let global_update_tx = self.global_update_tx.clone();
        let model_clone = model.clone();
        
        tokio::spawn(async move {
            let result = manager.run_stream(
                stream_id_for_task.clone(),
                llm_client,
                messages,
                model_clone,
                cancel_rx,
                update_tx_clone,
                global_update_tx,
            ).await;

            if let Err(e) = result {
                error!("Stream {} failed: {}", stream_id_for_task, e);
                let _ = manager.mark_stream_failed(&stream_id_for_task, &e.to_string()).await;
            }

            // Remove from active streams
            let mut streams = manager.active_streams.write().await;
            streams.remove(&stream_id_for_task);
        });

        Ok(stream_id.clone())
    }

    async fn run_stream(
        &self,
        stream_id: String,
        llm_client: Box<dyn LLMClient>,
        messages: Vec<serde_json::Value>,
        model: String,
        mut cancel_rx: mpsc::Receiver<()>,
        update_tx: broadcast::Sender<StreamingUpdate>,
        global_update_tx: broadcast::Sender<StreamingUpdate>,
    ) -> Result<(), AppError> {
        let mut accumulated_content = String::new();
        let mut total_tokens = 0;

        // Create the stream  
        let stream_result = llm_client.chat_stream(&model, messages).await;
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                let error_msg = format!("Failed to create stream: {}", e);
                let update = StreamingUpdate {
                    stream_id: stream_id.clone(),
                    content_delta: String::new(),
                    is_complete: true,
                    error: Some(error_msg.clone()),
                    tokens_used: None,
                };
                let _ = update_tx.send(update.clone());
                let _ = global_update_tx.send(update);
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

                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                content_delta: chunk.clone(),
                                is_complete: false,
                                error: None,
                                tokens_used: None, // We'll count tokens at the end
                            };

                            // Send to stream-specific subscribers
                            let _ = update_tx.send(update.clone());
                            // Send to global subscribers
                            let _ = global_update_tx.send(update);

                            // Update database periodically (every 100 chars to reduce DB load)
                            if accumulated_content.len() % 100 == 0 {
                                self.update_streaming_content(&stream_id, &accumulated_content, None).await?;
                            }
                        }
                        Ok(Some(Err(e))) => {
                            let error_msg = format!("Stream error: {}", e);
                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                content_delta: String::new(),
                                is_complete: true,
                                error: Some(error_msg.clone()),
                                tokens_used: Some(total_tokens),
                            };
                            let _ = update_tx.send(update.clone());
                            let _ = global_update_tx.send(update);
                            
                            self.mark_stream_failed(&stream_id, &error_msg).await?;
                            return Err(AppError::InternalServerError);
                        }
                        Ok(None) => {
                            // Stream completed successfully
                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                content_delta: String::new(),
                                is_complete: true,
                                error: None,
                                tokens_used: Some(total_tokens),
                            };
                            let _ = update_tx.send(update.clone());
                            let _ = global_update_tx.send(update);

                            self.mark_stream_completed(&stream_id, &accumulated_content, Some(total_tokens)).await?;
                            return Ok(());
                        }
                        Err(_) => {
                            // Timeout
                            let error_msg = "Stream timeout - no data received for 30 seconds".to_string();
                            let update = StreamingUpdate {
                                stream_id: stream_id.clone(),
                                content_delta: String::new(),
                                is_complete: true,
                                error: Some(error_msg.clone()),
                                tokens_used: Some(total_tokens),
                            };
                            let _ = update_tx.send(update.clone());
                            let _ = global_update_tx.send(update);

                            self.mark_stream_failed(&stream_id, &error_msg).await?;
                            return Err(AppError::InternalServerError);
                        }
                    }
                }
            }
        }
    }

    pub async fn cancel_stream(&self, stream_id: &str) -> Result<(), AppError> {
        let streams = self.active_streams.read().await;
        if let Some(active_stream) = streams.get(stream_id) {
            let _ = active_stream.cancel_tx.send(()).await;
            info!("Sent cancellation signal to stream {}", stream_id);
        }
        Ok(())
    }

    pub fn subscribe_to_stream(&self, stream_id: &str) -> Option<broadcast::Receiver<StreamingUpdate>> {
        // We need to use try_read since we can't await in a non-async context
        if let Ok(streams) = self.active_streams.try_read() {
            streams.get(stream_id).map(|stream| stream.update_tx.subscribe())
        } else {
            None
        }
    }

    pub fn subscribe_to_all_streams(&self) -> broadcast::Receiver<StreamingUpdate> {
        self.global_update_tx.subscribe()
    }

    pub async fn get_streaming_state(&self, stream_id: &str) -> Result<Option<StreamingState>, AppError> {
        let result = sqlx::query_as!(
            StreamingStateRow,
            "SELECT id, message_id, chat_id, user_id, status, content, provider, model, total_tokens, created_at, completed_at, error_message FROM streaming_states WHERE id = $1",
            stream_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(result.map(|row| row.into()))
    }

    pub async fn get_user_streaming_states(&self, user_id: &str, status: Option<StreamingStatus>) -> Result<Vec<StreamingState>, AppError> {
        let rows = if let Some(status) = status {
            sqlx::query_as!(
                StreamingStateRow,
                "SELECT id, message_id, chat_id, user_id, status, content, provider, model, total_tokens, created_at, completed_at, error_message FROM streaming_states WHERE user_id = $1 AND status = $2 ORDER BY created_at DESC",
                user_id,
                status.to_string()
            )
            .fetch_all(&self.db_pool)
            .await?
        } else {
            sqlx::query_as!(
                StreamingStateRow,
                "SELECT id, message_id, chat_id, user_id, status, content, provider, model, total_tokens, created_at, completed_at, error_message FROM streaming_states WHERE user_id = $1 ORDER BY created_at DESC",
                user_id
            )
            .fetch_all(&self.db_pool)
            .await?
        };

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    async fn save_streaming_state(&self, state: &StreamingState) -> Result<(), AppError> {
        sqlx::query!(
            "INSERT INTO streaming_states (id, message_id, chat_id, user_id, status, content, provider, model, total_tokens, created_at, completed_at, error_message) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            state.id,
            state.message_id,
            state.chat_id,
            state.user_id,
            state.status.to_string(),
            state.content,
            state.provider,
            state.model,
            state.total_tokens,
            state.created_at,
            state.completed_at,
            state.error_message
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn update_streaming_content(&self, stream_id: &str, content: &str, total_tokens: Option<i32>) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE streaming_states SET content = $1, total_tokens = $2 WHERE id = $3",
            content,
            total_tokens,
            stream_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn mark_stream_completed(&self, stream_id: &str, final_content: &str, total_tokens: Option<i32>) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE streaming_states SET status = 'completed', content = $1, total_tokens = $2, completed_at = NOW() WHERE id = $3",
            final_content,
            total_tokens,
            stream_id
        )
        .execute(&self.db_pool)
        .await?;

        // Update the actual message with the final content
        if let Ok(Some(state)) = self.get_streaming_state(stream_id).await {
            let _ = sqlx::query!(
                "UPDATE messages SET content = $1 WHERE id = $2",
                final_content,
                state.message_id
            )
            .execute(&self.db_pool)
            .await;
        }

        info!("Stream {} completed successfully", stream_id);
        Ok(())
    }

    async fn mark_stream_failed(&self, stream_id: &str, error_message: &str) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE streaming_states SET status = 'failed', error_message = $1, completed_at = NOW() WHERE id = $2",
            error_message,
            stream_id
        )
        .execute(&self.db_pool)
        .await?;

        warn!("Stream {} failed: {}", stream_id, error_message);
        Ok(())
    }

    async fn mark_stream_cancelled(&self, stream_id: &str) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE streaming_states SET status = 'cancelled', completed_at = NOW() WHERE id = $1",
            stream_id
        )
        .execute(&self.db_pool)
        .await?;

        info!("Stream {} cancelled", stream_id);
        Ok(())
    }

    pub async fn cleanup_old_streams(&self, older_than_hours: i32) -> Result<u64, AppError> {
        let interval = format!("{} hours", older_than_hours);
        let result = sqlx::query("DELETE FROM streaming_states WHERE created_at < NOW() - INTERVAL $1 AND status IN ('completed', 'failed', 'cancelled')")
            .bind(interval)
        .execute(&self.db_pool)
        .await?;

        info!("Cleaned up {} old streaming states", result.rows_affected());
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct StreamingStateRow {
    id: String,
    message_id: String,
    chat_id: String,
    user_id: String,
    status: String,
    content: String,
    provider: String,
    model: String,
    total_tokens: Option<i32>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
}

impl From<StreamingStateRow> for StreamingState {
    fn from(row: StreamingStateRow) -> Self {
        Self {
            id: row.id,
            message_id: row.message_id,
            chat_id: row.chat_id,
            user_id: row.user_id,
            status: row.status.parse().unwrap_or(StreamingStatus::Failed),
            content: row.content,
            provider: row.provider,
            model: row.model,
            total_tokens: row.total_tokens,
            created_at: row.created_at,
            completed_at: row.completed_at,
            error_message: row.error_message,
        }
    }
}