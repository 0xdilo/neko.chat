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

pub struct MessageUtils;

impl MessageUtils {
    /// Separates system messages from user messages (used by OpenAI and Anthropic)
    pub fn separate_system_messages(messages: Vec<Value>) -> (Option<String>, Vec<Value>) {
        let mut system_prompt = None;
        let mut user_messages = Vec::new();

        for message in messages {
            if let Some(role) = message.get("role").and_then(|r| r.as_str()) {
                if role == "system" {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        system_prompt = Some(content.to_string());
                    }
                } else {
                    user_messages.push(message);
                }
            }
        }

        (system_prompt, user_messages)
    }

    /// Transforms messages to Gemini format
    pub fn transform_to_gemini_format(messages: Vec<Value>) -> Vec<Value> {
        let mut contents = Vec::new();
        
        for message in messages {
            if let (Some(role), Some(content)) = (
                message.get("role").and_then(|r| r.as_str()),
                message.get("content").and_then(|c| c.as_str()),
            ) {
                if role == "system" {
                    continue; // Skip system messages for Gemini
                }
                
                let gemini_role = if role == "user" { "user" } else { "model" };
                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": [{"text": content}]
                }));
            }
        }
        
        contents
    }
}