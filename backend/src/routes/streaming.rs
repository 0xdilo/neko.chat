use crate::handlers::streaming_handler::{
    cancel_stream, get_streaming_state, get_user_streaming_states,
};
use crate::AppState;
use axum::{
    routing::{delete, get},
    Router,
};

pub fn create_streaming_routes() -> Router<AppState> {
    Router::new()
        .route("/api/streaming/states", get(get_user_streaming_states))
        .route("/api/streaming/states/:stream_id", get(get_streaming_state))
        .route("/api/streaming/states/:stream_id/cancel", delete(cancel_stream))
}