use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handlers::{
        enhanced_llm_handler::{enhanced_stream_message, enhanced_regenerate_response},
        enhanced_ws_handler::enhanced_websocket_handler,
    },
    AppState,
};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        // Enhanced WebSocket endpoint
        .route("/ws/v2", get(enhanced_websocket_handler))
        
        // Enhanced streaming endpoints
        .route("/api/v2/chats/:chat_id/stream", post(enhanced_stream_message))
        .route("/api/v2/chats/:chat_id/regenerate", post(enhanced_regenerate_response))
}