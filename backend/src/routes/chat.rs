use crate::{handlers::{chat_handler, llm_handler}, AppState};
use axum::{
    routing::{delete, get, patch, post},
    Router,
};

pub fn create_chat_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/chats",
            get(chat_handler::list_chats).post(chat_handler::create_chat),
        )
        .route(
            "/api/chats/:id",
            patch(chat_handler::update_chat).delete(chat_handler::delete_chat),
        )
        .route(
            "/api/chats/:id/messages",
            get(chat_handler::get_messages).post(llm_handler::send_message),
        )
        .route(
            "/api/chats/:chat_id/messages/:message_id",
            patch(chat_handler::update_message).delete(chat_handler::delete_single_message),
        )
        .route(
            "/api/chats/:chat_id/messages/:message_id/and-subsequent",
            delete(chat_handler::delete_message_and_subsequent),
        )
        .route(
            "/api/chats/:chat_id/messages/:message_id/subsequent",
            delete(chat_handler::delete_subsequent_messages),
        )
        .route(
            "/api/chats/:id/messages/bulk",
            post(chat_handler::bulk_insert_messages),
        )
        .route("/api/chats/:id/stream", post(llm_handler::stream_message))
        .route(
            "/api/chats/:id/regenerate",
            post(llm_handler::regenerate_response),
        )
        .route(
            "/api/chats/:id/parallel",
            post(llm_handler::parallel_llm_query),
        )
}