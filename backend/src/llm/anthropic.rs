use crate::error::AppError;
use super::types::*;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;

pub async fn fetch_anthropic_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
    let client = Client::new();
    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if !response.status().is_success() {
        return Err(AppError::InternalServerError);
    }

    let models_response: AnthropicModelsResponse = response
        .json()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let normalized_models = models_response
        .data
        .into_iter()
        .map(|model| NormalizedModel {
            id: model.id.clone(),
            name: model.display_name.unwrap_or(model.id.clone()),
            provider: "anthropic".to_string(),
            description: None,
            context_length: Some(200000),
            created: if let Some(created_at) = model.created_at {
                created_at
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0)
            } else {
                0
            },
        })
        .collect();

    Ok(normalized_models)
}

pub struct AnthropicClient {
    api_key: String,
    client: Client,
}

impl AnthropicClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }

    fn separate_system_messages(&self, messages: Vec<Value>) -> (Option<String>, Vec<Value>) {
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
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn chat(&self, model: &str, messages: Vec<Value>) -> Result<String, AppError> {
        let (system_prompt, user_messages) = self.separate_system_messages(messages);

        let mut request_body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": user_messages,
        });

        if let Some(system) = system_prompt {
            request_body["system"] = serde_json::Value::String(system);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Anthropic API error: status {}, body: {}",
                status,
                error_text
            );

            // Parse error message from Anthropic API response
            let error_message = if !error_text.is_empty() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    error_json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Service Error")
                        .to_string()
                } else {
                    error_text
                }
            } else {
                match status.as_u16() {
                    429 => "Rate limit exceeded".to_string(),
                    401 => "Invalid API key".to_string(),
                    400 => "Bad request".to_string(),
                    503 => "Service temporarily unavailable".to_string(),
                    _ => format!("HTTP {}", status.as_u16()),
                }
            };

            return Err(AppError::LLMProviderError {
                provider: "Anthropic".to_string(),
                status_code: Some(status.as_u16()),
                message: error_message,
            });
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        let content = response_json
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .unwrap_or_default();

        Ok(content.to_string())
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError> {
        let mut max_tokens = 4096;
        if model.contains("reasoning") {
            tracing::info!("Using Anthropic reasoning model: {}", model);
            max_tokens = 8192;
        }

        let (system_prompt, user_messages) = self.separate_system_messages(messages);

        let mut request_body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": user_messages,
            "stream": true,
        });

        if let Some(system) = system_prompt {
            request_body["system"] = serde_json::Value::String(system);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Anthropic streaming API error: status {}, body: {}",
                status,
                error_text
            );

            // Parse error message from Anthropic API response
            let error_message = if !error_text.is_empty() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    error_json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Service Error")
                        .to_string()
                } else {
                    error_text
                }
            } else {
                match status.as_u16() {
                    429 => "Rate limit exceeded".to_string(),
                    401 => "Invalid API key".to_string(),
                    400 => "Bad request".to_string(),
                    503 => "Service temporarily unavailable".to_string(),
                    _ => format!("HTTP {}", status.as_u16()),
                }
            };

            return Err(AppError::LLMProviderError {
                provider: "Anthropic".to_string(),
                status_code: Some(status.as_u16()),
                message: error_message,
            });
        }

        let byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            let mut inner_stream = byte_stream;
            while let Some(chunk_result) = inner_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Anthropic stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };

                for line in chunk.split(|&b| b == b'\n') {
                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        let data_str = String::from_utf8_lossy(data);
                        // tracing::debug!("Anthropic stream data: {}", data_str);

                        if data == b"[DONE]" {
                            break;
                        }

                        if let Ok(parsed) = serde_json::from_slice::<Value>(data) {
                            // tracing::debug!("Anthropic parsed JSON: {:?}", parsed);

                            if let Some(event_type) = parsed.get("type").and_then(|t| t.as_str()) {
                                if event_type == "content_block_delta" {
                                    if let Some(delta) = parsed.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                                        yield Ok(delta.to_string());
                                    }
                                }
                            }
                        } else {
                            tracing::warn!("Failed to parse Anthropic JSON: {}", data_str);
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn chat_stream_with_web_search(
        &self,
        model: &str,
        messages: Vec<Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError> {
        let mut max_tokens = 4096;
        if model.contains("reasoning") {
            tracing::info!("Using Anthropic reasoning model: {}", model);
            max_tokens = 8192;
        }

        let (system_prompt, user_messages) = self.separate_system_messages(messages);

        let mut request_body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": user_messages,
            "stream": true,
            "tools": [
                {
                    "type": "web_search_20250305",
                    "name": "web_search",
                    "max_uses": 5
                }
            ]
        });

        if let Some(system) = system_prompt {
            request_body["system"] = serde_json::Value::String(system);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Anthropic web search streaming API error: status {}, body: {}",
                status,
                error_text
            );
            return Err(AppError::InternalServerError);
        }

        let byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            let mut inner_stream = byte_stream;
            while let Some(chunk_result) = inner_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Anthropic web search stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };

                for line in chunk.split(|&b| b == b'\n') {
                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        let data_str = String::from_utf8_lossy(data);

                        if data == b"[DONE]" {
                            break;
                        }

                        if let Ok(parsed) = serde_json::from_slice::<Value>(data) {
                            if let Some(event_type) = parsed.get("type").and_then(|t| t.as_str()) {
                                if event_type == "content_block_delta" {
                                    if let Some(delta) = parsed.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                                        yield Ok(delta.to_string());
                                    }
                                }
                            }
                        } else {
                            tracing::warn!("Failed to parse Anthropic web search JSON: {}", data_str);
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn supports_web_search(&self) -> bool {
        true
    }
}