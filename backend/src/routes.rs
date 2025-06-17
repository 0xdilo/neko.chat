use crate::{
    handlers::{
        auth_handler, chat_handler, key_handler, llm_handler, settings_handler, ws_handler,
    },
    AppState,
};
use axum::routing::delete;
use axum::{
    routing::{get, patch, post},
    Router,
};

pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/api/auth/register", post(auth_handler::register))
        .route("/api/auth/login", post(auth_handler::login))
        .route("/api/auth/google", get(auth_handler::google_auth_url))
        .route(
            "/api/auth/google/callback",
            get(auth_handler::google_callback),
        )
        .route("/api/auth/me", get(auth_handler::get_me))
        .route("/api/auth/profile", get(auth_handler::get_me))
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
        .route(
            "/api/keys",
            post(key_handler::add_key).get(key_handler::list_keys),
        )
        .route(
            "/api/keys/:provider",
            get(key_handler::get_key).delete(key_handler::delete_key),
        )
        .route(
            "/api/settings",
            get(settings_handler::get_settings).put(settings_handler::update_settings),
        )
        .route(
            "/api/settings/api-keys",
            get(settings_handler::get_api_keys),
        )
        .route(
            "/api/settings/prompts",
            get(settings_handler::get_system_prompts).post(settings_handler::create_system_prompt),
        )
        .route(
            "/api/settings/prompts/active",
            get(settings_handler::get_active_system_prompts),
        )
        .route(
            "/api/settings/prompts/:id",
            patch(settings_handler::update_system_prompt)
                .delete(settings_handler::delete_system_prompt),
        )
        .route(
            "/api/settings/prompts/:id/activate",
            post(settings_handler::set_active_prompt),
        )
        .route(
            "/api/settings/prompts/:id/toggle",
            post(settings_handler::toggle_active_prompt),
        )
        .route(
            "/api/settings/models/fetch",
            post(settings_handler::fetch_models_for_provider),
        )
        .route(
            "/api/settings/models",
            get(settings_handler::get_user_models)
                .put(settings_handler::update_user_model_preferences),
        )
        .route(
            "/api/settings/models/enabled",
            get(settings_handler::get_enabled_models_for_user),
        )
        .route(
            "/api/settings/models/toggle",
            post(settings_handler::toggle_model_enabled),
        )
        .route("/ws", get(ws_handler::websocket_handler))
        .with_state(app_state)
}
