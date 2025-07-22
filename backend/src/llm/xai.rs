use crate::error::AppError;
use async_stream::stream;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;

use super::{LLMClient, NormalizedModel, OpenAiResponse, OpenAiStreamResponse};
use super::common::HttpClientUtils;

pub struct XaiClient {
    api_key: String,
    client: Client,
}

impl XaiClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for XaiClient {
    async fn chat(&self, model: &str, messages: Vec<Value>) -> Result<String, AppError> {
        let response = self
            .client
            .post("https://api.x.ai/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            tracing::error!("xai api error: {:?}", response.text().await);
            return Err(AppError::InternalServerError);
        }

        let openai_response = response.json::<OpenAiResponse>().await.map_err(|e| {
            tracing::error!("failed to parse xai response: {}", e);
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
        let request_json = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        if model.contains("reasoning") || model.contains("mini") {
            tracing::info!("Using reasoning model: {}", model);
        }

        let response = self
            .client
            .post("https://api.x.ai/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request_json)
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        let response = HttpClientUtils::handle_api_error(response, "XAI").await?;
        
        let byte_stream = response.bytes_stream();
        let stream = async_stream::stream! {
            let mut inner_stream = byte_stream;
            while let Some(chunk_result) = inner_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("XAI stream chunk error: {}", e);
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
        self.chat_stream(model, messages).await
    }

    fn supports_web_search(&self) -> bool {
        false
    }
}

pub async fn fetch_xai_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
    // XAI uses OpenAI-compatible API, so we can use similar model fetching
    let client = Client::new();
    let response = client
        .get("https://api.x.ai/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if !response.status().is_success() {
        return Err(AppError::InternalServerError);
    }

    // For now, return static models as XAI might not have a models endpoint yet
    let normalized_models = vec![
        NormalizedModel {
            id: "grok-beta".to_string(),
            name: "Grok Beta".to_string(),
            provider: "xai".to_string(),
            description: Some("Grok's Beta model".to_string()),
            context_length: Some(131072),
            created: 1640995200, // Jan 1, 2022
        },
    ];

    Ok(normalized_models)
}