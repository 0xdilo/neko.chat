use crate::{handlers::auth_handler, AppState};
use axum::{
    routing::{get, post},
    Router,
};

pub fn create_auth_routes() -> Router<AppState> {
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
}