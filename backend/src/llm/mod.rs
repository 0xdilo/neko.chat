use crate::error::AppError;

pub mod types;
pub mod common;
pub mod openai;
pub mod anthropic;
pub mod openrouter;
pub mod xai;
pub mod gemini;

pub use types::{LLMClient, NormalizedModel, OpenAiResponse, OpenAiStreamResponse};
pub use openai::{OpenAIClient, fetch_openai_models};
pub use anthropic::{AnthropicClient, fetch_anthropic_models};
pub use openrouter::{OpenRouterClient, fetch_openrouter_models};
pub use xai::{XaiClient, fetch_xai_models};
pub use gemini::{GeminiClient, fetch_gemini_models};

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