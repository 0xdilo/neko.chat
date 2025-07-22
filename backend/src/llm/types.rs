use crate::error::AppError;
use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, model: &str, messages: Vec<Value>) -> Result<String, AppError>;

    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError>;

    async fn chat_stream_with_web_search(
        &self,
        model: &str,
        messages: Vec<Value>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AppError>> + Send>>, AppError>;

    fn supports_web_search(&self) -> bool;
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

#[derive(Deserialize, Serialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
    pub created: i64,
    pub description: String,
    pub context_length: i32,
    pub pricing: serde_json::Value,
}

#[derive(Serialize)]
pub struct NormalizedModel {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
    pub context_length: Option<i32>,
    pub created: i64,
}

#[derive(Deserialize, Serialize)]
pub struct AnthropicModel {
    pub id: String,
    pub display_name: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct AnthropicModelsResponse {
    pub data: Vec<AnthropicModel>,
}

#[derive(Deserialize, Serialize)]
pub struct GeminiModel {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GeminiModelsResponse {
    pub models: Vec<GeminiModel>,
}

// Shared OpenAI-compatible response structs
#[derive(Deserialize)]
pub struct OpenAiChoice {
    pub message: OpenAiMessage,
}

#[derive(Deserialize)]
pub struct OpenAiMessage {
    pub content: String,
}

#[derive(Deserialize)]
pub struct OpenAiResponse {
    pub choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
pub struct OpenAiStreamChoice {
    pub delta: OpenAiStreamDelta,
}

#[derive(Deserialize)]
pub struct OpenAiStreamDelta {
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenAiStreamResponse {
    pub choices: Vec<OpenAiStreamChoice>,
}