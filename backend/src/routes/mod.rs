use crate::AppState;
use axum::Router;

pub mod auth;
pub mod chat;
pub mod key;
pub mod settings;
pub mod websocket;

pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .merge(auth::create_auth_routes())
        .merge(chat::create_chat_routes())
        .merge(key::create_key_routes())
        .merge(settings::create_settings_routes())
        .merge(websocket::create_websocket_routes())
        .with_state(app_state)
}