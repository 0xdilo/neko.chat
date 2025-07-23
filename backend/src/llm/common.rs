use crate::error::AppError;
use reqwest::{Response, StatusCode};
use serde_json::Value;

pub struct HttpClientUtils;

impl HttpClientUtils {
    /// Common error handling for API responses
    pub async fn handle_api_error(
        response: Response,
        provider: &str,
    ) -> Result<Response, AppError> {
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("{} API error: status {}, body: {}", provider, status, error_text);

            let error_message = Self::parse_error_message(&error_text, status);

            return Err(AppError::LLMProviderError {
                provider: provider.to_string(),
                status_code: Some(status.as_u16()),
                message: error_message,
            });
        }
        Ok(response)
    }

    fn parse_error_message(error_text: &str, status: StatusCode) -> String {
        if !error_text.is_empty() {
            if let Ok(error_json) = serde_json::from_str::<Value>(error_text) {
                error_json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Service Error")
                    .to_string()
            } else {
                error_text.to_string()
            }
        } else {
            match status.as_u16() {
                429 => "Rate limit exceeded".to_string(),
                401 => "Invalid API key".to_string(),
                400 => "Bad request".to_string(),
                503 => "Service temporarily unavailable".to_string(),
                _ => format!("HTTP {}", status.as_u16()),
            }
        }
    }
}

