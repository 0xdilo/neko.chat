// Export all API services
export { api, endpoints, withErrorHandling, withLoading } from './client.js';
export { authAPI } from './auth.js';
export { chatAPI } from './chats.js';
export { llmAPI, streamUtils } from './llm.js';
export { modelsAPI, modelUtils } from './models.js';
export { settingsAPI } from './settings.js';
export { 
  websocket, 
  wsClient, 
  wsConnected, 
  wsConnecting, 
  wsError, 
  WS_MESSAGE_TYPES 
} from './websocket.js';

// Re-export for convenience
export default {
  auth: authAPI,
  chats: chatAPI,
  llm: llmAPI,
  models: modelsAPI,
  settings: settingsAPI,
  websocket,
};

// Initialize API connections
export function initializeAPI() {
  // WebSocket will auto-connect if authenticated
  // Other initialization can be added here
}

// API utilities
export const apiUtils = {
  // Format API errors for display
  formatError(error) {
    if (typeof error === 'string') return error;
    if (error.message) return error.message;
    if (error.error) return error.error;
    return 'An unexpected error occurred';
  },

  // Check if error is network-related
  isNetworkError(error) {
    return error.message?.includes('fetch') || 
           error.message?.includes('network') ||
           error.message?.includes('timeout') ||
           !navigator.onLine;
  },

  // Check if error is authentication-related
  isAuthError(error) {
    return error.status === 401 || 
           error.status === 403 ||
           error.message?.includes('auth') ||
           error.message?.includes('token');
  },

  // Retry API call with exponential backoff
  async retry(apiCall, maxAttempts = 3, baseDelay = 1000) {
    let lastError;
    
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        return await apiCall();
      } catch (error) {
        lastError = error;
        
        // Don't retry auth errors or client errors
        if (this.isAuthError(error) || (error.status >= 400 && error.status < 500)) {
          throw error;
        }
        
        if (attempt < maxAttempts) {
          const delay = baseDelay * Math.pow(2, attempt - 1);
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }
    
    throw lastError;
  },

  // Debounce API calls
  debounce(func, delay = 300) {
    let timeoutId;
    return function (...args) {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => func.apply(this, args), delay);
    };
  },

  // Throttle API calls
  throttle(func, limit = 1000) {
    let inThrottle;
    return function (...args) {
      if (!inThrottle) {
        func.apply(this, args);
        inThrottle = true;
        setTimeout(() => inThrottle = false, limit);
      }
    };
  },

  // Batch multiple API calls
  async batch(apiCalls, concurrency = 3) {
    const results = [];
    const executing = [];
    
    for (const apiCall of apiCalls) {
      const promise = apiCall().then(result => {
        executing.splice(executing.indexOf(promise), 1);
        return result;
      });
      
      results.push(promise);
      executing.push(promise);
      
      if (executing.length >= concurrency) {
        await Promise.race(executing);
      }
    }
    
    return Promise.all(results);
  },

  // Cache API responses
  createCache(ttl = 300000) { // 5 minutes default
    const cache = new Map();
    
    return {
      get(key) {
        const item = cache.get(key);
        if (!item) return null;
        
        if (Date.now() > item.expiry) {
          cache.delete(key);
          return null;
        }
        
        return item.data;
      },
      
      set(key, data) {
        cache.set(key, {
          data,
          expiry: Date.now() + ttl
        });
      },
      
      delete(key) {
        cache.delete(key);
      },
      
      clear() {
        cache.clear();
      }
    };
  }
};