use crate::{auth, error::AppError, AppState, streaming_manager::StreamingManager};
use axum::{
    extract::{
        ws::{WebSocket, Message as WsMsg},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WsAuthQuery {
    token: String,
}

async fn authenticate_ws(app_state: &AppState, token: &str) -> Result<auth::Claims, AppError> {
    let decoding_key = DecodingKey::from_secret(app_state.config.jwt_secret.as_ref());
    let validation = Validation::default();
    let token_data = decode::<auth::Claims>(token, &decoding_key, &validation)
        .map_err(|_| AppError::Unauthorized)?;
    Ok(token_data.claims)
}

pub async fn enhanced_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(auth): Query<WsAuthQuery>,
) -> impl IntoResponse {
    let user_id = match authenticate_ws(&state, &auth.token).await {
        Ok(claims) => claims.sub,
        Err(e) => return e.into_response(),
    };

    ws.on_upgrade(move |socket| handle_enhanced_socket(socket, user_id, state))
}

async fn handle_enhanced_socket(socket: WebSocket, user_id: String, state: AppState) {
    let socket_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection {} for user {}", socket_id, user_id);

    // Register connection with streaming manager
    if let Some(streaming_manager) = &state.streaming_manager {
        streaming_manager.register_connection(user_id.clone(), socket_id.clone()).await;
    }

    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to broadcast channel
    let mut broadcast_rx = state.tx.subscribe();
    
    // Clone necessary state for tasks
    let user_id_clone = user_id.clone();
    let streaming_manager = state.streaming_manager.clone();
    let broadcast_tx = state.tx.clone();
    let state_clone = state.clone();

    // Spawn heartbeat task
    let heartbeat_handle = tokio::spawn({
        let user_id = user_id.clone();
        let socket_id = socket_id.clone();
        let streaming_manager = streaming_manager.clone();
        
        async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Some(ref manager) = streaming_manager {
                    manager.update_connection_heartbeat(&user_id, &socket_id).await;
                }
            }
        }
    });

    // Spawn task to handle incoming messages from client
    let incoming_handle = tokio::spawn({
        let user_id = user_id.clone();
        let socket_id = socket_id.clone();
        let streaming_manager = streaming_manager.clone();
        
        async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(WsMsg::Text(text)) => {
                        // Parse and handle client messages
                        if let Ok(ws_message) = serde_json::from_str::<crate::ws_messages::WsMessage>(&text) {
                            match ws_message.message_type {
                                crate::ws_messages::WsMessageType::Ping => {
                                    // Update heartbeat on ping
                                    if let Some(ref manager) = streaming_manager {
                                        manager.update_connection_heartbeat(&user_id, &socket_id).await;
                                    }
                                }
                                crate::ws_messages::WsMessageType::StreamingResume => {
                                    // Handle stream resume request
                                    if let Some(stream_id) = ws_message.data.get("stream_id").and_then(|v| v.as_str()) {
                                        handle_stream_resume(&streaming_manager, stream_id, &user_id, &broadcast_tx).await;
                                    }
                                }
                                crate::ws_messages::WsMessageType::RequestStreamResume => {
                                    // Handle request for active stream resumption
                                    info!("Received RequestStreamResume from user {}", user_id);
                                    if let Some(ref manager) = streaming_manager {
                                        manager.check_and_resume_active_streams(&user_id).await;
                                    }
                                }
                                _ => {
                                    // Handle other message types if needed
                                }
                            }
                        }
                    }
                    Ok(WsMsg::Close(_)) => {
                        info!("WebSocket {} closing", socket_id);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket {} error: {}", socket_id, e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    });

    // Spawn task to send messages to this client
    let outgoing_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle broadcast messages
                Ok(msg) = broadcast_rx.recv() => {
                    // Check if this is a streaming message (any streaming-related message type)
                    let is_streaming_message = matches!(msg.message_type, 
                        crate::ws_messages::WsMessageType::StreamingUpdate | 
                        crate::ws_messages::WsMessageType::StreamingStart |
                        crate::ws_messages::WsMessageType::StreamingResume |
                        crate::ws_messages::WsMessageType::StreamingComplete |
                        crate::ws_messages::WsMessageType::StreamingError
                    );
                    
                    if matches!(msg.message_type, crate::ws_messages::WsMessageType::StreamingResume) {
                        info!("WebSocket handler received StreamingResume message: {:?}", msg.data);
                    }
                    
                    if is_streaming_message {
                        // Streaming messages are filtered by user_id in the message
                        if let Some(msg_user_id) = msg.data.get("user_id").and_then(|v| v.as_str()) {
                            info!("Received streaming message for user_id: {}, connected user: {}, match: {}", 
                                msg_user_id, user_id_clone, msg_user_id == user_id_clone);
                            if msg_user_id == user_id_clone {
                                let message_json = match serde_json::to_string(&msg) {
                                    Ok(json) => json,
                                    Err(e) => {
                                        error!("Failed to serialize message: {}", e);
                                        continue;
                                    }
                                };
                                
                                if sender.send(WsMsg::Text(message_json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    } else {
                        // For non-streaming messages, check chat ownership
                        let should_send = if let Some(chat_id_val) = msg.data.get("chat_id") {
                            if let Some(chat_id) = chat_id_val.as_str() {
                                !chat_id.is_empty() && user_owns_chat(chat_id, &user_id_clone, &state_clone).await.unwrap_or(false)
                            } else {
                                false
                            }
                        } else {
                            // Send messages without chat_id (system messages, etc.)
                            true
                        };

                        if should_send {
                            let message_json = match serde_json::to_string(&msg) {
                                Ok(json) => json,
                                Err(e) => {
                                    error!("Failed to serialize message: {}", e);
                                    continue;
                                }
                            };
                            
                            if sender.send(WsMsg::Text(message_json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                
                // Send periodic pings
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    let ping_msg = crate::ws_messages::WsMessage::new_ping();
                    let ping_json = match serde_json::to_string(&ping_msg) {
                        Ok(json) => json,
                        Err(_) => continue,
                    };
                    
                    if sender.send(WsMsg::Text(ping_json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Wait for any handle to complete
    tokio::select! {
        _ = heartbeat_handle => {
            warn!("Heartbeat task ended for socket {}", socket_id);
        }
        _ = incoming_handle => {
            warn!("Incoming task ended for socket {}", socket_id);
        }
        _ = outgoing_handle => {
            warn!("Outgoing task ended for socket {}", socket_id);
        }
    }

    // Cleanup: unregister connection
    if let Some(ref manager) = streaming_manager {
        manager.unregister_connection(&user_id, &socket_id).await;
    }
    
    info!("WebSocket {} disconnected for user {}", socket_id, user_id);
}

async fn user_owns_chat(chat_id: &str, user_id: &str, state: &AppState) -> Result<bool, sqlx::Error> {
    let result: Result<(String,), sqlx::Error> = sqlx::query_as(
        "SELECT user_id FROM chats WHERE id = $1"
    )
    .bind(chat_id)
    .fetch_one(&state.db_pool)
    .await;
    
    match result {
        Ok((owner_id,)) => Ok(owner_id == user_id),
        Err(_) => Ok(false),
    }
}

async fn handle_stream_resume(
    streaming_manager: &Option<StreamingManager>,
    stream_id: &str,
    user_id: &str,
    ws_broadcast_tx: &tokio::sync::broadcast::Sender<crate::ws_messages::WsMessage>,
) {
    if let Some(manager) = streaming_manager {
        match manager.get_stream_resume_data(stream_id, user_id).await {
            Ok(Some((state, _chunks))) => {
                info!("Resuming stream {} for user {} from chunk {}", 
                    stream_id, user_id, state.last_chunk_index);
                
                // Send resume notification with accumulated content via the broadcast channel
                let resume_msg = crate::ws_messages::WsMessage::new_streaming_resume(
                    stream_id.to_string(),
                    state.message_id.clone(),
                    state.chat_id.clone(),
                    state.last_chunk_index,
                    state.content.clone(),
                );
                
                // Broadcast the resume message (it will be filtered by user_id in the WebSocket handler)
                if let Err(e) = ws_broadcast_tx.send(resume_msg) {
                    error!("Failed to send resume message for stream {}: {}", stream_id, e);
                }
                
                // The actual chunks will be sent through the normal streaming update mechanism
            }
            Ok(None) => {
                warn!("No resume data found for stream {} and user {}", stream_id, user_id);
            }
            Err(e) => {
                error!("Error getting resume data for stream {}: {}", stream_id, e);
            }
        }
    }
}