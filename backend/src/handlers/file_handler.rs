use crate::{
    auth::Claims,
    database::Attachment,
    error::AppError,
    AppState,
};
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use std::fs;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB
const ALLOWED_TYPES: &[&str] = &[
    "image/jpeg", "image/png", "image/gif", "image/webp",
    "text/plain", "application/pdf"
];

#[derive(Deserialize, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
}

pub async fn upload_file(
    State(app_state): State<AppState>,
    claims: Claims,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let pool = &app_state.db_pool;
    let user_id = claims.sub;
    let upload_base_dir = PathBuf::from(app_state.upload_dir);
    
    // Create user-specific uploads directory if it doesn't exist
    let uploads_dir = upload_base_dir.join(&user_id);
    fs::create_dir_all(&uploads_dir).map_err(|_| AppError::InternalServerError)?;

    while let Some(field) = multipart.next_field().await.map_err(|_| AppError::BadRequest("Invalid multipart data".to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        
        if name != "file" {
            continue;
        }

        let file_name = field.file_name()
            .ok_or_else(|| AppError::BadRequest("No filename provided".to_string()))?
            .to_string();
        
        let content_type = field.content_type()
            .ok_or_else(|| AppError::BadRequest("No content type provided".to_string()))?
            .to_string();

        // Validate file type
        if !ALLOWED_TYPES.contains(&content_type.as_str()) {
            return Err(AppError::BadRequest("File type not supported".to_string()));
        }

        let data = field.bytes().await.map_err(|_| AppError::BadRequest("Failed to read file data".to_string()))?;
        
        // Validate file size
        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::BadRequest("File too large".to_string()));
        }

        // Generate unique filename
        let file_id = Uuid::new_v4().to_string();
        let extension = std::path::Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        let stored_filename = if extension.is_empty() {
            file_id.clone()
        } else {
            format!("{}.{}", file_id, extension)
        };
        
        let file_path = uploads_dir.join(&stored_filename);
        let relative_path = file_path.to_str().unwrap_or("").to_string();

        // Write file to disk
        let mut file = File::create(&file_path).await.map_err(|_| AppError::InternalServerError)?;
        file.write_all(&data).await.map_err(|_| AppError::InternalServerError)?;

        // Save attachment record to database (without message_id for now)
        let attachment = sqlx::query_as::<_, Attachment>(
            r#"
            INSERT INTO attachments (id, message_id, user_id, file_name, file_type, file_size, file_path)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&file_id)
        .bind(None::<String>) // Null message_id, will be updated when message is sent
        .bind(&user_id)
        .bind(&file_name)
        .bind(&content_type)
        .bind(data.len() as i64)
        .bind(&relative_path)
        .fetch_one(pool)
        .await?;

        return Ok(Json(UploadResponse {
            id: attachment.id,
            file_name: attachment.file_name,
            file_type: attachment.file_type,
            file_size: attachment.file_size,
        }));
    }

    Err(AppError::BadRequest("No file found in request".to_string()))
}

pub async fn serve_file(
    State(app_state): State<AppState>,
    claims: Claims,
    Path(file_id): Path<String>,
) -> Result<Response, AppError> {
    let pool = &app_state.db_pool;
    let user_id = claims.sub;

    // Get attachment info from database
    let attachment = sqlx::query_as::<_, Attachment>(
        "SELECT * FROM attachments WHERE id = $1 AND user_id = $2"
    )
    .bind(&file_id)
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::NotFound)?;

    // Open file
    let file = File::open(&attachment.file_path).await.map_err(|_| AppError::NotFound)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // Build response with appropriate headers
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &attachment.file_type)
        .header(header::CONTENT_LENGTH, attachment.file_size.to_string())
        .header(
            header::CONTENT_DISPOSITION,
            format!(r#"inline; filename="{}""#, attachment.file_name)
        );
