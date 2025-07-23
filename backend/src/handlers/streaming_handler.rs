use crate::{
    auth::Claims,
    error::AppError,
    streaming_manager::{StreamingManager, StreamingStatus},
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct StreamingStateQuery {
    status: Option<String>,
}

#[derive(Serialize)]
pub struct StreamingStateResponse {
    pub streaming_states: Vec<crate::streaming_manager::StreamingState>,
}

#[derive(Serialize)]
pub struct StreamingStateDetailResponse {
    pub streaming_state: Option<crate::streaming_manager::StreamingState>,
}

pub async fn get_user_streaming_states(
    State(streaming_manager): State<Option<StreamingManager>>,
    claims: Claims,
    Query(query): Query<StreamingStateQuery>,
) -> Result<Json<StreamingStateResponse>, AppError> {
    let user_id = claims.sub;
    
    let manager = streaming_manager
        .ok_or_else(|| AppError::InternalServerError)?;
    
    let status = if let Some(status_str) = query.status {
        Some(status_str.parse::<StreamingStatus>()
            .map_err(|_| AppError::BadRequest("Invalid status".to_string()))?)
    } else {
        None
    };

    let streaming_states = manager
        .get_user_streaming_states(&user_id, status)
        .await?;

    Ok(Json(StreamingStateResponse { streaming_states }))
}

pub async fn get_streaming_state(
    State(streaming_manager): State<Option<StreamingManager>>,
    claims: Claims,
    Path(stream_id): Path<String>,
) -> Result<Json<StreamingStateDetailResponse>, AppError> {
    let manager = streaming_manager
        .ok_or_else(|| AppError::InternalServerError)?;
        
    let streaming_state = manager
        .get_streaming_state(&stream_id)
        .await?;

    // Verify the user owns this stream
    if let Some(ref state) = streaming_state {
        if state.user_id != claims.sub {
            return Err(AppError::Unauthorized);
        }
    }

    Ok(Json(StreamingStateDetailResponse { streaming_state }))
}

pub async fn cancel_stream(
    State(streaming_manager): State<Option<StreamingManager>>,
    claims: Claims,
    Path(stream_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let manager = streaming_manager
        .ok_or_else(|| AppError::InternalServerError)?;
        
    // Verify the user owns this stream
    let streaming_state = manager
        .get_streaming_state(&stream_id)
        .await?;

    if let Some(state) = streaming_state {
        if state.user_id != claims.sub {
            return Err(AppError::Unauthorized);
        }
        
        manager.cancel_stream(&stream_id).await?;
        Ok(Json(serde_json::json!({"success": true})))
    } else {
        Err(AppError::NotFound)
    }
}