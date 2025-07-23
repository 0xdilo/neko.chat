use crate::error::AppError;
use super::{LLMClient, NormalizedModel, OpenAiResponse, OpenAiStreamResponse};
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

pub async fn fetch_openai_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
    let client = Client::new();
    let response = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if !response.status().is_success() {
        return Err(AppError::InternalServerError);
    }

    let models_response: ModelsResponse = response
        .json()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let mut normalized_models: Vec<NormalizedModel> = models_response
        .data
        .into_iter()
        .filter(|model| {
            let id_lower = model.id.to_lowercase();
            let is_excluded = id_lower.contains("audio")
                || id_lower.contains("whisper")
                || id_lower.contains("tts")
                || id_lower.contains("speech")
                || id_lower.contains("image")
                || id_lower.contains("dall-e")
                || id_lower.contains("vision")
                || id_lower.contains("embedding")
                || id_lower.contains("ada")
                || id_lower.contains("moderation")
                || id_lower.contains("edit")
                || id_lower.contains("search")
                || id_lower.contains("similarity")
                || id_lower.ends_with("-001")
                || id_lower.contains("babbage")
                || id_lower.contains("curie")
                || (id_lower.contains("davinci")
                    && !id_lower.starts_with("text-davinci")
                    && id_lower != "davinci-002")
                || id_lower.contains("canary")
                || id_lower.contains("playground")
                || id_lower.contains("ft:")
                || id_lower.contains("realtime");

            !is_excluded
        })
        .map(|model| NormalizedModel {
            id: model.id.clone(),
            name: model.id.clone(),
            provider: "openai".to_string(),
            description: None,
            context_length: None,
            created: model.created,
        })
        .collect();

    normalized_models.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(normalized_models)
}

pub struct OpenAIClient {
    api_key: String,
    client: Client,
}

impl OpenAIClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }

}


#[derive(Deserialize, Serialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

#[derive(Deserialize, Serialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<Model>,
}


#[async_trait]
impl LLMClient for OpenAIClient {
    async fn chat(&self, model: &str, messages: Vec<Value>) -> Result<String, AppError> {
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("openai api error: {:?}", error_text);

            // Parse error message from OpenAI API response
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
                provider: "OpenAI".to_string(),
                status_code: Some(status.as_u16()),
                message: error_message,
            });
        }

        let openai_response = response.json::<OpenAiResponse>().await.map_err(|e| {
            tracing::error!("failed to parse openai response: {}", e);
            AppError::InternalServerError
        })?;

        Ok(openai_response
            .choices
            .get(0)
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError> {
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "OpenAI streaming API error: status {}, body: {}",
                status,
                error_text
            );

            // Parse error message from OpenAI API response
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
                provider: "OpenAI".to_string(),
                status_code: Some(status.as_u16()),
                message: error_message,
            });
        }

        let byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            let mut inner_stream = byte_stream;
            let mut buffer = Vec::with_capacity(8192); // Pre-allocate buffer

            while let Some(chunk_result) = inner_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("OpenAI stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };

                buffer.extend_from_slice(&chunk);

                // Process complete lines from buffer
                let mut start = 0;
                while let Some(newline_pos) = buffer[start..].iter().position(|&b| b == b'\n') {
                    let line_end = start + newline_pos;
                    let line = &buffer[start..line_end];

                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        if data == b"[DONE]" {
                            return;
                        }
                        if let Ok(parsed) = serde_json::from_slice::<OpenAiStreamResponse>(data) {
                            if let Some(content) = parsed.choices.get(0).and_then(|c| c.delta.content.as_ref()) {
                                if !content.is_empty() {
                                    yield Ok(content.clone());
                                }
                            }
                        }
                    }
                    start = line_end + 1;
                }

                // Keep remaining incomplete data in buffer
                if start < buffer.len() {
                    buffer.drain(..start);
                } else {
                    buffer.clear();
                }
                
                // Prevent buffer from growing too large
                if buffer.len() > 32768 {
                    buffer.clear();
                    tracing::warn!("OpenAI stream buffer overflow, clearing");
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
        self.chat_stream(model, messages).await
    }

    fn supports_web_search(&self) -> bool {
        false
    }
}