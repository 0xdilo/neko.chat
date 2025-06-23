use crate::{auth, error::AppError, AppState};
use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WsAuthQuery {
    token: String,
}

// Authenticates the WebSocket connection via a query parameter token.
async fn authenticate_ws(app_state: &AppState, token: &str) -> Result<String, AppError> {
    let decoding_key = DecodingKey::from_secret(app_state.config.jwt_secret.as_ref());
    let validation = Validation::default();
    let token_data = decode::<auth::Claims>(token, &decoding_key, &validation)
        .map_err(|_| AppError::Unauthorized)?;
    Ok(token_data.claims.sub)
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(auth): Query<WsAuthQuery>,
) -> impl IntoResponse {
    let user_id = match authenticate_ws(&state, &auth.token).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, user_id: String) {
    let mut rx = state.tx.subscribe();

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let chat_owner_id: Result<(String,), sqlx::Error> =
                sqlx::query_as("SELECT user_id FROM chats WHERE id = $1")
                    .bind(&msg.chat_id)
                    .fetch_one(&state.db_pool)
                    .await;

            if let Ok((owner_id,)) = chat_owner_id {
                if owner_id == user_id {
                    let payload = serde_json::to_string(&msg).unwrap();
                    if socket.send(WsMessage::Text(payload)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
}
