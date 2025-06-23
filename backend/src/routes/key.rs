use crate::{handlers::key_handler, AppState};
use axum::{
    routing::{get, post},
    Router,
};

pub fn create_key_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/keys",
            post(key_handler::add_key).get(key_handler::list_keys),
        )
        .route(
            "/api/keys/:provider",
            get(key_handler::get_key).delete(key_handler::delete_key),
        )
}

