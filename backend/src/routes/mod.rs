use crate::AppState;
use axum::Router;

pub mod auth;
pub mod chat;
pub mod key;
pub mod settings;
pub mod enhanced_streaming;

pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .merge(auth::create_auth_routes())
        .merge(chat::create_chat_routes())
        .merge(key::create_key_routes())
        .merge(settings::create_settings_routes())
        .merge(enhanced_streaming::create_routes())
        .with_state(app_state)
}