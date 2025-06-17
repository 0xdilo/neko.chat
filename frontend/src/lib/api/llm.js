import { api, endpoints, withErrorHandling } from './client.js';

// LLM API service for AI model interactions
export const llmAPI = {
  // Send chat completion request
  async chatCompletion(messages, options = {}) {
    return withErrorHandling(
      () => api.post(endpoints.llm.chat, {
        messages,
        model: options.model || 'gpt-4',
        temperature: options.temperature || 0.7,
        maxTokens: options.maxTokens || 2048,
        systemPrompt: options.systemPrompt,
        stream: false
      }),
      'Failed to get AI response.'
    );
  },

  // Send streaming chat completion request
  async streamChatCompletion(messages, options = {}, onChunk) {
    try {
      const reader = await api.stream(endpoints.llm.stream, {
        messages,
        model: options.model || 'gpt-4',
        temperature: options.temperature || 0.7,
        maxTokens: options.maxTokens || 2048,
        systemPrompt: options.systemPrompt,
        stream: true
      });

      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || ''; // Keep incomplete line in buffer

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            
            if (data === '[DONE]') {
              return;
            }

            try {
              const chunk = JSON.parse(data);
              if (onChunk) {
                onChunk(chunk);
              }
            } catch (error) {
              console.warn('Failed to parse streaming chunk:', error);
            }
          }
        }
      }
    } catch (error) {
      console.error('Streaming error:', error);
      throw new Error('Failed to stream AI response.');
    }
  },

  // Get available models
  async getModels() {
    return withErrorHandling(
      () => api.get(endpoints.llm.models),
      'Failed to load available models.'
    );
  },

  // Get model information
  async getModelInfo(modelId) {
    return withErrorHandling(
      () => api.get(`${endpoints.llm.models}/${modelId}`),
      'Failed to load model information.'
    );
  },

  // Get usage statistics
  async getUsage(period = 'month') {
    return withErrorHandling(
      () => api.get(endpoints.llm.usage, { period }),
      'Failed to load usage statistics.'
    );
  },

  // Get token count estimate
  async estimateTokens(text, model = 'gpt-4') {
    return withErrorHandling(
      () => api.post('/api/llm/estimate-tokens', { text, model }),
      'Failed to estimate tokens.'
    );
  },

  // Validate API key
  async validateApiKey(provider, apiKey) {
    return withErrorHandling(
      () => api.post('/api/llm/validate-key', { provider, apiKey }),
      'Failed to validate API key.'
    );
  },

  // Generate title for conversation
  async generateTitle(messages, model = 'gpt-3.5-turbo') {
    return withErrorHandling(
      () => api.post('/api/llm/generate-title', { messages, model }),
      'Failed to generate title.'
    );
  },

  // Summarize conversation
  async summarizeConversation(messages, options = {}) {
    return withErrorHandling(
      () => api.post('/api/llm/summarize', {
        messages,
        model: options.model || 'gpt-3.5-turbo',
        maxLength: options.maxLength || 200
      }),
      'Failed to summarize conversation.'
    );
  },

  // Get content moderation
  async moderateContent(text) {
    return withErrorHandling(
      () => api.post('/api/llm/moderate', { text }),
      'Failed to moderate content.'
    );
  },

  // Generate embeddings
  async generateEmbeddings(texts, model = 'text-embedding-ada-002') {
    return withErrorHandling(
      () => api.post('/api/llm/embeddings', { texts, model }),
      'Failed to generate embeddings.'
    );
  },

  // Search similar messages using embeddings
  async searchSimilar(query, chatId, limit = 10) {
    return withErrorHandling(
      () => api.post('/api/llm/search-similar', { query, chatId, limit }),
      'Failed to search similar messages.'
    );
  },

  // Function calling / tool use
  async functionCall(messages, functions, options = {}) {
    return withErrorHandling(
      () => api.post('/api/llm/function-call', {
        messages,
        functions,
        model: options.model || 'gpt-4',
        temperature: options.temperature || 0.7
      }),
      'Failed to execute function call.'
    );
  },

  // Image analysis (for vision models)
  async analyzeImage(imageData, prompt, options = {}) {
    return withErrorHandling(
      () => api.post('/api/llm/analyze-image', {
        image: imageData,
        prompt,
        model: options.model || 'gpt-4-vision-preview',
        maxTokens: options.maxTokens || 1000
      }),
      'Failed to analyze image.'
    );
  },

  // Code completion
  async completeCode(code, language, options = {}) {
    return withErrorHandling(
      () => api.post('/api/llm/complete-code', {
        code,
        language,
        model: options.model || 'gpt-4',
        temperature: options.temperature || 0.1
      }),
      'Failed to complete code.'
    );
  },

  // Text-to-speech
  async textToSpeech(text, options = {}) {
    return withErrorHandling(
      () => api.post('/api/llm/tts', {
        text,
        voice: options.voice || 'alloy',
        model: options.model || 'tts-1',
        speed: options.speed || 1.0
      }),
      'Failed to generate speech.'
    );
  },

  // Speech-to-text
  async speechToText(audioData, options = {}) {
    const formData = new FormData();
    formData.append('audio', audioData);
    formData.append('model', options.model || 'whisper-1');
    formData.append('language', options.language || 'en');
    
    return withErrorHandling(
      () => api.upload('/api/llm/stt', formData),
      'Failed to transcribe speech.'
    );
  },

  // Cancel ongoing request
  cancelRequest(requestId) {
    // This would work with AbortController in the actual implementation
    console.log('Cancelling request:', requestId);
  },
};

// Utility functions for working with streaming responses
export const streamUtils = {
  // Parse streaming response chunk
  parseChunk(chunk) {
    if (chunk.choices && chunk.choices[0]) {
      const choice = chunk.choices[0];
      
      if (choice.delta && choice.delta.content) {
        return {
          type: 'content',
          content: choice.delta.content,
          finished: choice.finish_reason !== null
        };
      }
      
      if (choice.finish_reason) {
        return {
          type: 'finish',
          reason: choice.finish_reason
        };
      }
    }
    
    return {
      type: 'unknown',
      raw: chunk
    };
  },

  // Accumulate streaming content
  accumulator() {
    let content = '';
    
    return {
      add(chunk) {
        const parsed = this.parseChunk(chunk);
        if (parsed.type === 'content') {
          content += parsed.content;
        }
        return parsed;
      },
      
      getContent() {
        return content;
      },
      
      reset() {
        content = '';
      }
    };
  }
};