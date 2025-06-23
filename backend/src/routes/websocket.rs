use crate::{handlers::ws_handler, AppState};
use axum::{routing::get, Router};

pub fn create_websocket_routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler::websocket_handler))
}