use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    InternalServerError,
    BadRequest(String),
    Unauthorized,
    NotFound,
    DatabaseError(sqlx::Error),
    JwtError(jsonwebtoken::errors::Error),
    PasswordHashError(bcrypt::BcryptError),
    LLMProviderError {
        provider: String,
        status_code: Option<u16>,
        message: String,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error_message = match self {
            AppError::InternalServerError => "Internal Server Error",
            AppError::BadRequest(msg) => msg.as_str(),
            AppError::Unauthorized => "Unauthorized",
            AppError::NotFound => "Not Found",
            AppError::DatabaseError(_) => "Database operation failed",
            AppError::JwtError(_) => "Invalid token",
            AppError::PasswordHashError(_) => "Could not process request",
            AppError::LLMProviderError { provider, status_code, message } => {
                if let Some(code) = status_code {
                    return write!(f, "{} (HTTP {}): {}", provider, code, message);
                } else {
                    return write!(f, "{}: {}", provider, message);
                }
            }
        };
        write!(f, "{}", error_message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::DatabaseError(ref e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AppError::JwtError(ref e) => {
                tracing::error!("JWT error: {}", e);
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            AppError::PasswordHashError(ref e) => {
                tracing::error!("Password hash error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AppError::LLMProviderError { ref provider, status_code, ref message } => {
                tracing::error!("LLM Provider error - {}: {}", provider, message);
                // For HTTP responses, convert to appropriate status code
                let response_status = status_code
                    .and_then(|code| StatusCode::from_u16(code).ok())
                    .unwrap_or(StatusCode::BAD_GATEWAY);
                (response_status, self.to_string())
            }
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::DatabaseError(e)
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AppError::JwtError(e)
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AppError::PasswordHashError(e)
    }
}
