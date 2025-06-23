use crate::{handlers::settings_handler, AppState};
use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

pub fn create_settings_routes() -> Router<AppState> {
    Router::new()
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
}