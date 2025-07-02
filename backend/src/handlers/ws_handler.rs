use crate::{auth, error::AppError, database::Message, AppState};
use axum::{
    extract::{
        ws::WebSocket,
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;

#[derive(Deserialize)]
pub struct WsAuthQuery {
    token: String,
}

// Authenticates the WebSocket connection via a query parameter token.
async fn authenticate_ws(app_state: &AppState, token: &str) -> Result<auth::Claims, AppError> {
    let decoding_key = DecodingKey::from_secret(app_state.config.jwt_secret.as_ref());
    let validation = Validation::default();
    let token_data = decode::<auth::Claims>(token, &decoding_key, &validation)
        .map_err(|_| AppError::Unauthorized)?;
    Ok(token_data.claims)
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(auth): Query<WsAuthQuery>,
) -> impl IntoResponse {
    let user_id = match authenticate_ws(&state, &auth.token).await {
        Ok(claims) => claims.sub,
        Err(e) => return e.into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, user_id, state))
}

async fn handle_socket(socket: WebSocket, user_id: String, state: AppState) {
    let (mut sender, mut _receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Spawn task to send messages to this client
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Use cached user_id instead of querying database for every message
            if msg.chat_id.is_empty() || user_owns_chat(&msg.chat_id, &user_id, &state).await.unwrap_or(false) {
                let message_json = serde_json::to_string(&msg).unwrap();
                if sender.send(axum::extract::ws::Message::Text(message_json)).await.is_err() {
                    break;
                }
            }
        }
    });
}

// Helper function to check chat ownership
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
