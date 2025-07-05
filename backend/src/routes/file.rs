use crate::handlers::file_handler;
use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;

pub fn create_file_routes() -> Router<AppState> {
    Router::new()
        .route("/upload", post(file_handler::upload_file))
        .route("/:file_id", get(file_handler::serve_file))
        .route("/:file_id/content", get(file_handler::get_file_content))
}
