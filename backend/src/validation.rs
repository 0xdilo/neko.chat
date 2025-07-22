use crate::error::AppError;

pub struct MessageValidator;

impl MessageValidator {
    /// Validates message content
    pub fn validate_content(content: &str) -> Result<(), AppError> {
        // Check if content is not empty (after trimming)
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("Message content cannot be empty".to_string()));
        }

        // Check maximum length (reasonable limit for LLM input)
        const MAX_CONTENT_LENGTH: usize = 100_000; // 100KB
        if content.len() > MAX_CONTENT_LENGTH {
            return Err(AppError::BadRequest(format!(
                "Message content too long. Maximum {} characters allowed",
                MAX_CONTENT_LENGTH
            )));
        }

        // Check for potentially harmful content patterns
        Self::validate_content_safety(content)?;

        Ok(())
    }

    /// Validates chat title
    pub fn validate_title(title: &str) -> Result<(), AppError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("Title cannot be empty".to_string()));
        }

        const MAX_TITLE_LENGTH: usize = 200;
        if title.len() > MAX_TITLE_LENGTH {
            return Err(AppError::BadRequest(format!(
                "Title too long. Maximum {} characters allowed",
                MAX_TITLE_LENGTH
            )));
        }

        // Basic sanitization check
        if title.contains('\0') || title.contains('\r') || title.contains('\n') {
            return Err(AppError::BadRequest("Title contains invalid characters".to_string()));
        }

        Ok(())
    }

    /// Validates system prompt
    pub fn validate_system_prompt(prompt: &str) -> Result<(), AppError> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("System prompt cannot be empty".to_string()));
        }

        const MAX_SYSTEM_PROMPT_LENGTH: usize = 10_000;
        if prompt.len() > MAX_SYSTEM_PROMPT_LENGTH {
            return Err(AppError::BadRequest(format!(
                "System prompt too long. Maximum {} characters allowed",
                MAX_SYSTEM_PROMPT_LENGTH
            )));
        }

        Ok(())
    }

    /// Validates role for bulk message import
    pub fn validate_role(role: &str) -> Result<(), AppError> {
        match role {
            "user" | "assistant" | "system" => Ok(()),
            _ => Err(AppError::BadRequest(format!("Invalid role '{}'. Must be 'user', 'assistant', or 'system'", role)))
        }
    }

    /// Validates provider name
    pub fn validate_provider(provider: &str) -> Result<(), AppError> {
        match provider {
            "openai" | "anthropic" | "openrouter" | "xai" | "gemini" => Ok(()),
            _ => Err(AppError::BadRequest(format!("Invalid provider '{}'. Supported providers: openai, anthropic, openrouter, xai, gemini", provider)))
        }
    }

    /// Validates model name format
    pub fn validate_model(model: &str) -> Result<(), AppError> {
        if model.trim().is_empty() {
            return Err(AppError::BadRequest("Model name cannot be empty".to_string()));
        }

        const MAX_MODEL_LENGTH: usize = 100;
        if model.len() > MAX_MODEL_LENGTH {
            return Err(AppError::BadRequest(format!(
                "Model name too long. Maximum {} characters allowed",
                MAX_MODEL_LENGTH
            )));
        }

        // Basic format validation (alphanumeric, dash, underscore, dot, colon)
        if !model.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')) {
            return Err(AppError::BadRequest("Model name contains invalid characters. Only alphanumeric, dash, underscore, dot, and colon allowed".to_string()));
        }

        Ok(())
    }

    /// Validates bulk messages payload
    pub fn validate_bulk_messages(messages: &[crate::handlers::chat_handler::BulkMessagePayload]) -> Result<(), AppError> {
        if messages.is_empty() {
            return Err(AppError::BadRequest("Messages array cannot be empty".to_string()));
        }

        const MAX_BULK_MESSAGES: usize = 100;
        if messages.len() > MAX_BULK_MESSAGES {
            return Err(AppError::BadRequest(format!(
                "Too many messages. Maximum {} messages allowed per bulk import",
                MAX_BULK_MESSAGES
            )));
        }

        for (i, message) in messages.iter().enumerate() {
            Self::validate_role(&message.role)
                .map_err(|e| AppError::BadRequest(format!("Message {}: {}", i + 1, e)))?;
            Self::validate_content(&message.content)
                .map_err(|e| AppError::BadRequest(format!("Message {}: {}", i + 1, e)))?;
        }

        Ok(())
    }

    /// Technical security validation (exploit prevention only)
    fn validate_content_safety(content: &str) -> Result<(), AppError> {
        // Check for null bytes that could cause issues with string handling
        if content.contains('\0') {
            return Err(AppError::BadRequest("Content contains null bytes".to_string()));
        }

        // Check for excessively long lines that might indicate buffer overflow attempts
        const MAX_LINE_LENGTH: usize = 50_000; // Increased limit
        for line in content.lines() {
            if line.len() > MAX_LINE_LENGTH {
                return Err(AppError::BadRequest(format!(
                    "Individual line too long (max {} characters per line)",
                    MAX_LINE_LENGTH
                )));
            }
        }

        // Only check for the most egregious injection attempts that would never be legitimate chat content
        // Note: We're being very conservative here since this is a chat app where users discuss code
        
        // Check for data URLs with executable content (potential XSS vector)
        if content.contains("data:text/html,") || content.contains("data:application/javascript,") {
            return Err(AppError::BadRequest(
                "Content contains data URLs with executable content".to_string()
            ));
        }

        // Check for obvious script injection attempts with immediate execution
        let dangerous_patterns = [
            "javascript:alert(",
            "javascript:eval(",
            "vbscript:execute(",
            "data:text/html,<script",
        ];

        let content_lower = content.to_lowercase();
        for pattern in dangerous_patterns {
            if content_lower.contains(pattern) {
                return Err(AppError::BadRequest(
                    "Content contains executable script injection patterns".to_string()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content() {
        // Valid content (including potentially controversial topics - should be allowed)
        assert!(MessageValidator::validate_content("Hello world").is_ok());
        assert!(MessageValidator::validate_content("Let's discuss politics and controversial topics").is_ok());
        assert!(MessageValidator::validate_content("This contains strong language but should be fine").is_ok());
        
        // Empty content
        assert!(MessageValidator::validate_content("").is_err());
        assert!(MessageValidator::validate_content("   ").is_err());
        
        // Too long content
        let long_content = "a".repeat(100_001);
        assert!(MessageValidator::validate_content(&long_content).is_err());
        
        // Security issues - should be rejected
        assert!(MessageValidator::validate_content("Hello\0world").is_err());
        assert!(MessageValidator::validate_content("javascript:alert('malicious')").is_err());
        assert!(MessageValidator::validate_content("data:text/html,<script>alert('xss')</script>").is_err());
        
        // Valid code discussion content should be allowed
        assert!(MessageValidator::validate_content("I want to discuss <script> tags in web development").is_ok());
        assert!(MessageValidator::validate_content("Here's some JavaScript code: function test() { alert('hello'); }").is_ok());
        assert!(MessageValidator::validate_content("The onclick event handler is useful").is_ok());
        assert!(MessageValidator::validate_content("Let's talk about XSS prevention techniques").is_ok());
    }

    #[test]
    fn test_validate_role() {
        assert!(MessageValidator::validate_role("user").is_ok());
        assert!(MessageValidator::validate_role("assistant").is_ok());
        assert!(MessageValidator::validate_role("system").is_ok());
        assert!(MessageValidator::validate_role("invalid").is_err());
    }

    #[test]
    fn test_validate_provider() {
        assert!(MessageValidator::validate_provider("openai").is_ok());
        assert!(MessageValidator::validate_provider("anthropic").is_ok());
        assert!(MessageValidator::validate_provider("invalid").is_err());
    }
}