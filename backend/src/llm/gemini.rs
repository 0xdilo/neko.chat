use crate::error::AppError;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

use super::{LLMClient, NormalizedModel};

#[derive(Deserialize, Serialize)]
pub struct GeminiModel {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GeminiModelsResponse {
    pub models: Vec<GeminiModel>,
}

#[derive(Deserialize, Debug)]
struct GeminiTextPart {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GeminiContent {
    parts: Vec<GeminiTextPart>,
}

#[derive(Deserialize, Debug)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

pub struct GeminiClient {
    api_key: String,
    client: Client,
}

impl GeminiClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for GeminiClient {
    async fn chat(&self, model: &str, messages: Vec<Value>) -> Result<String, AppError> {
        let mut contents = Vec::new();

        for message in messages {
            if let (Some(role), Some(content)) = (
                message.get("role").and_then(|r| r.as_str()),
                message.get("content").and_then(|c| c.as_str()),
            ) {
                if role == "system" {
                    continue;
                }
                let gemini_role = if role == "user" { "user" } else { "model" };
                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": [{"text": content}]
                }));
            }
        }

        let response = self
            .client
            .post(&format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            ))
            .json(&serde_json::json!({
                "contents": contents,
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("gemini api error: {:?}", error_text);

            // Parse error message from Gemini API response
            let error_message = if !error_text.is_empty() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    error_json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Service Unavailable")
                        .to_string()
                } else {
                    error_text
                }
            } else {
                match status.as_u16() {
                    503 => "Service Unavailable".to_string(),
                    429 => "Rate limit exceeded".to_string(),
                    401 => "Invalid API key".to_string(),
                    400 => "Bad request".to_string(),
                    _ => format!("HTTP {}", status.as_u16()),
                }
            };

            return Err(AppError::LLMProviderError {
                provider: "Gemini".to_string(),
                status_code: Some(status.as_u16()),
                message: error_message,
            });
        }

        let gemini_response = response.json::<GeminiResponse>().await.map_err(|e| {
            tracing::error!("failed to parse gemini response: {}", e);
            AppError::InternalServerError
        })?;

        Ok(gemini_response
            .candidates
            .get(0)
            .and_then(|c| c.content.parts.get(0))
            .map(|p| p.text.clone())
            .unwrap_or_default())
    }

    fn supports_web_search(&self) -> bool {
        true
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError> {
        let mut contents = Vec::new();

        for message in messages {
            if let (Some(role), Some(content)) = (
                message.get("role").and_then(|r| r.as_str()),
                message.get("content").and_then(|c| c.as_str()),
            ) {
                if role == "system" {
                    continue;
                }
                let gemini_role = if role == "user" { "user" } else { "model" };
                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": [{"text": content}]
                }));
            }
        }

        let response = self
            .client
            .post(&format!("https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}", model, self.api_key))
            .json(&serde_json::json!({
                "contents": contents,
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        println!("{:?}", response);
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Gemini streaming API error: status {}, body: {}",
                status,
                error_text
            );

            // Parse error message from Gemini API response
            let error_message = if !error_text.is_empty() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    error_json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Service Unavailable")
                        .to_string()
                } else {
                    error_text
                }
            } else {
                match status.as_u16() {
                    503 => "Service Unavailable".to_string(),
                    429 => "Rate limit exceeded".to_string(),
                    401 => "Invalid API key".to_string(),
                    400 => "Bad request".to_string(),
                    _ => format!("HTTP {}", status.as_u16()),
                }
            };

            return Err(AppError::LLMProviderError {
                provider: "Gemini".to_string(),
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
                        tracing::error!("Gemini stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };
                println!("{:?}", chunk);

                for line in chunk.split(|&b| b == b'\n') {
                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        let data_str = String::from_utf8_lossy(data);

                        if data.is_empty() || data == b"[DONE]" {
                            continue;
                        }

                        // tracing::debug!("Gemini stream data: {}", data_str);

                        if let Ok(parsed) = serde_json::from_slice::<Value>(data) {
                            // tracing::debug!("Gemini parsed JSON: {:?}", parsed);

                            if let Some(candidates) = parsed.get("candidates").and_then(|c| c.as_array()) {
                                if let Some(candidate) = candidates.get(0) {
                                    if let Some(content) = candidate.get("content") {
                                        if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                            if let Some(part) = parts.get(0) {
                                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                    if !text.is_empty() {
                                                        yield Ok(text.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Only warn if it's not just grounding metadata
                            if !data_str.contains("groundingMetadata") {
                                tracing::warn!("Failed to parse Gemini JSON: {}", data_str);
                            }
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
        let mut contents = Vec::new();

        for message in messages {
            if let (Some(role), Some(content)) = (
                message.get("role").and_then(|r| r.as_str()),
                message.get("content").and_then(|c| c.as_str()),
            ) {
                if role == "system" {
                    continue;
                }
                let gemini_role = if role == "user" { "user" } else { "model" };
                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": [{"text": content}]
                }));
            }
        }

        let response = self
            .client
            .post(&format!("https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}", model, self.api_key))
            .json(&serde_json::json!({
                "contents": contents,
                "tools": [
                    {
                        "google_search": {}
                    }
                ],
 
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Gemini web search streaming API error: status {}, body: {}",
                status,
                error_text
            );

            // Parse error message from Gemini API response
            let error_message = if !error_text.is_empty() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    error_json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Service Unavailable")
                        .to_string()
                } else {
                    error_text
                }
            } else {
                match status.as_u16() {
                    503 => "Service Unavailable".to_string(),
                    429 => "Rate limit exceeded".to_string(),
                    401 => "Invalid API key".to_string(),
                    400 => "Bad request".to_string(),
                    _ => format!("HTTP {}", status.as_u16()),
                }
            };

            return Err(AppError::LLMProviderError {
                provider: "Gemini".to_string(),
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
                        tracing::error!("Gemini web search stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };

                for line in chunk.split(|&b| b == b'\n') {
                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        let data_str = String::from_utf8_lossy(data);

                        if data.is_empty() || data == b"[DONE]" {
                            continue;
                        }

                        if let Ok(parsed) = serde_json::from_slice::<Value>(data) {
                            if let Some(candidates) = parsed.get("candidates").and_then(|c| c.as_array()) {
                                if let Some(candidate) = candidates.get(0) {
                                    if let Some(content) = candidate.get("content") {
                                        if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                            if let Some(part) = parts.get(0) {
                                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                    if !text.is_empty() {
                                                        yield Ok(text.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Only warn if it's not just grounding metadata
                            if !data_str.contains("groundingMetadata") {
                                tracing::warn!("Failed to parse Gemini web search JSON: {}", data_str);
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

pub async fn fetch_gemini_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
    let client = Client::new();
    let response = client
        .get(&format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            api_key
        ))
        .send()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if !response.status().is_success() {
        return Err(AppError::InternalServerError);
    }

    let models_response: GeminiModelsResponse = response
        .json()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let normalized_models = models_response
        .models
        .into_iter()
        .filter(|model| {
            model.name.contains("gemini")
                && !model.name.contains("embedding")
                && !model.name.contains("vision")
        })
        .map(|model| {
            let model_id = model
                .name
                .split('/')
                .last()
                .unwrap_or(&model.name)
                .to_string();
            NormalizedModel {
                id: model_id.clone(),
                name: format!(
                    "Gemini {}",
                    model_id.replace("gemini-", "").replace("-", " ")
                ),
                provider: "gemini".to_string(),
                description: model.description,
                context_length: Some(32768),
                created: 0,
            }
        })
        .collect();

    Ok(normalized_models)
}