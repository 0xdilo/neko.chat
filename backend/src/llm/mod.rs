use crate::error::AppError;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
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

pub fn get_llm_client(provider: &str, api_key: &str) -> Result<Box<dyn LLMClient>, AppError> {
    match provider {
        "openai" => Ok(Box::new(OpenAIClient::new(api_key))),
        "anthropic" => Ok(Box::new(AnthropicClient::new(api_key))),
        "openrouter" => Ok(Box::new(OpenRouterClient::new(api_key))),
        "xai" => Ok(Box::new(XaiClient::new(api_key))),
        "gemini" => Ok(Box::new(GeminiClient::new(api_key))),
        _ => Err(AppError::BadRequest(format!(
            "provider '{}' is not supported.",
            provider
        ))),
    }
}

pub async fn fetch_available_models(
    provider: &str,
    api_key: &str,
) -> Result<Vec<NormalizedModel>, AppError> {
    match provider {
        "openai" => fetch_openai_models(api_key).await,
        "anthropic" => fetch_anthropic_models(api_key).await,
        "openrouter" => fetch_openrouter_models(api_key).await,
        "xai" => fetch_xai_models(api_key).await,
        "gemini" => fetch_gemini_models(api_key).await,
        _ => Err(AppError::BadRequest(format!(
            "provider '{}' is not supported for model fetching.",
            provider
        ))),
    }
}

async fn fetch_openai_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
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

async fn fetch_anthropic_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
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

async fn fetch_xai_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
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

    let models_response: ModelsResponse = response
        .json()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let normalized_models = models_response
        .data
        .into_iter()
        .map(|model| NormalizedModel {
            id: model.id.clone(),
            name: format!("Grok {}", model.id.replace("grok-", "").replace("-", " ")),
            provider: "xai".to_string(),
            description: None,
            context_length: Some(131072),
            created: model.created,
        })
        .collect();

    Ok(normalized_models)
}

async fn fetch_gemini_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
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

async fn fetch_openrouter_models(api_key: &str) -> Result<Vec<NormalizedModel>, AppError> {
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

pub struct OpenAIClient {
    api_key: String,
    client: Client,
}

impl OpenAIClient {
    fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }
}

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
            let mut buffer = Vec::new();

            while let Some(chunk_result) = inner_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("stream chunk error: {}", e);
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

pub struct AnthropicClient {
    api_key: String,
    client: Client,
}

impl AnthropicClient {
    fn new(api_key: &str) -> Self {
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

pub struct OpenRouterClient {
    api_key: String,
    client: Client,
}

impl OpenRouterClient {
    fn new(api_key: &str) -> Self {
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
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
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
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
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

pub struct XaiClient {
    api_key: String,
    client: Client,
}

impl XaiClient {
    fn new(api_key: &str) -> Self {
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

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "xAI streaming API error: status {}, body: {}",
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
        self.chat_stream(model, messages).await
    }

    fn supports_web_search(&self) -> bool {
        false
    }
}

pub struct GeminiClient {
    api_key: String,
    client: Client,
}

impl GeminiClient {
    fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }
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
                "generationConfig": {
                    "temperature": 0.7,
                    "topP": 0.8,
                    "topK": 40,
                    "maxOutputTokens": 8192,
                }
            }))
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

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
                "generationConfig": {
                    "temperature": 0.7,
                    "topP": 0.8,
                    "topK": 40,
                    "maxOutputTokens": 8192,
                }
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
