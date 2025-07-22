use crate::error::AppError;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

use super::{LLMClient, NormalizedModel, OpenAiResponse, OpenAiStreamResponse};

#[derive(Deserialize, Serialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
    pub created: i64,
    pub description: String,
    pub context_length: i32,
    pub pricing: serde_json::Value,
}

pub async fn fetch_openrouter_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
    let client = Client::new();
    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if !response.status().is_success() {
        return Err(AppError::InternalServerError);
    }

    let models: Vec<OpenRouterModel> = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| AppError::InternalServerError)?
        .get("data")
        .and_then(|data| serde_json::from_value(data.clone()).ok())
        .unwrap_or_default();

    let normalized_models = models
        .into_iter()
        .map(|model| NormalizedModel {
            id: model.id.clone(),
            name: model.name,
            provider: "openrouter".to_string(),
            description: Some(model.description),
            context_length: Some(model.context_length),
            created: model.created,
        })
        .collect();

    Ok(normalized_models)
}

pub struct OpenRouterClient {
    api_key: String,
    client: Client,
}

impl OpenRouterClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for OpenRouterClient {
    async fn chat(&self, model: &str, messages: Vec<Value>) -> Result<String, AppError> {
        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("OpenRouter API request failed: {}", e);
                AppError::InternalServerError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "OpenRouter API error: status {}, body: {}",
                status,
                error_text
            );
            return Err(AppError::InternalServerError);
        }

        let openai_response = response.json::<OpenAiResponse>().await.map_err(|e| {
            tracing::error!("failed to parse openrouter response: {}", e);
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
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("OpenRouter API request failed: {}", e);
                AppError::InternalServerError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "OpenRouter streaming API error: status {}, body: {}",
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
                        tracing::error!("stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };

                for line in chunk.split(|&b| b == b'\n') {
                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        if data == b"[DONE]" {
                            break;
                        }
                        if let Ok(parsed) = serde_json::from_slice::<OpenAiStreamResponse>(data) {
                            if let Some(content) = parsed.choices.get(0).and_then(|c| c.delta.content.as_ref()) {
                                yield Ok(content.clone());
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
        let model_with_web = if model.contains(":online") {
            model.to_string()
        } else {
            format!("{}:online", model)
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": model_with_web,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            tracing::error!(
                "openrouter web search streaming api error: {:?}",
                response.text().await
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
                        tracing::error!("openrouter web search stream chunk error: {}", e);
                        yield Err(AppError::InternalServerError);
                        break;
                    }
                };

                for line in chunk.split(|&b| b == b'\n') {
                    if line.starts_with(b"data: ") {
                        let data = &line[6..];
                        if data == b"[DONE]" {
                            break;
                        }
                        if let Ok(parsed) = serde_json::from_slice::<OpenAiStreamResponse>(data) {
                            if let Some(content) = parsed.choices.get(0).and_then(|c| c.delta.content.as_ref()) {
                                yield Ok(content.clone());
                            }
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