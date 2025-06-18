import { browser } from "$app/environment";
import { showError, showWarning } from "$lib/stores/app.js";

// API configuration
const API_BASE_URL = browser
  ? window.location.origin.replace(":5173", ":8080") // Development proxy
  : "http://localhost:8080";

// Request timeout in milliseconds
const REQUEST_TIMEOUT = 30000;

// Create API client with authentication and error handling
class APIClient {
  constructor(baseURL = API_BASE_URL) {
    this.baseURL = baseURL;
    this.defaultHeaders = {
      "Content-Type": "application/json",
    };
  }

  // Get auth token from localStorage
  getAuthToken() {
    if (!browser) return null;
    return localStorage.getItem("neko-auth-token");
  }

  // Create headers with authentication
  getHeaders(customHeaders = {}) {
    const headers = { ...this.defaultHeaders, ...customHeaders };

    const token = this.getAuthToken();
    if (token) {
      headers.Authorization = `Bearer ${token}`;
    }

    return headers;
  }

  // Handle API response
  async handleResponse(response) {
    const contentType = response.headers.get("content-type");
    const isJson = contentType && contentType.includes("application/json");

    let data;
    try {
      data = isJson ? await response.json() : await response.text();
    } catch (error) {
      throw new Error("Failed to parse response");
    }

    if (!response.ok) {
      const errorMessage = isJson ? data.message || data.error : data;

      // Handle specific error codes
      switch (response.status) {
        case 401:
          // Unauthorized - redirect to login (unless disabled)
          if (browser && !this.currentRequest?.noRedirectOn401) {
            localStorage.removeItem("neko-auth-token");
            window.location.href = "/auth";
          }
          throw new Error("Authentication required");

        case 403:
          throw new Error("Access forbidden");

        case 404:
          throw new Error("Resource not found");

        case 429:
          showWarning("Rate limit exceeded. Please try again later.");
          throw new Error("Too many requests");

        case 500:
          showError("Server error. Please try again later.");
          throw new Error("Internal server error");

        default:
          throw new Error(errorMessage || `HTTP ${response.status}`);
      }
    }

    return data;
  }

  // Make HTTP request with timeout and error handling
  async request(endpoint, options = {}) {
    const url = `${this.baseURL}${endpoint}`;
    const config = {
      headers: this.getHeaders(options.headers),
      ...options,
    };

    // Extract custom options
    const { noRedirectOn401, ...fetchOptions } = config;

    // Add timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT);
    fetchOptions.signal = controller.signal;

    // Store request options for error handling
    this.currentRequest = { noRedirectOn401 };

    try {
      const response = await fetch(url, fetchOptions);
      clearTimeout(timeoutId);
      const result = await this.handleResponse(response);
      return result;
    } catch (error) {
      clearTimeout(timeoutId);
      console.error("API Request failed:", error);

      if (error.name === "AbortError") {
        throw new Error("Request timeout");
      }

      // Network error
      if (!navigator.onLine) {
        showWarning("You are offline. Please check your connection.");
        throw new Error("Network unavailable");
      }

      throw error;
    }
  }

  // HTTP methods
  async get(endpoint, params = {}) {
    const url = new URL(endpoint, this.baseURL);
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        url.searchParams.append(key, value);
      }
    });

    return this.request(url.pathname + url.search);
  }

  async post(endpoint, data = {}) {
    return this.request(endpoint, {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async put(endpoint, data = {}) {
    return this.request(endpoint, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async patch(endpoint, data = {}) {
    return this.request(endpoint, {
      method: "PATCH",
      body: JSON.stringify(data),
    });
  }

  async delete(endpoint) {
    return this.request(endpoint, {
      method: "DELETE",
    });
  }

  // Upload file
  async upload(endpoint, formData) {
    return this.request(endpoint, {
      method: "POST",
      headers: this.getHeaders({ "Content-Type": undefined }), // Let browser set content-type for FormData
      body: formData,
    });
  }

  // Streaming request for chat completions
  async stream(endpoint, data = {}) {
    const url = `${this.baseURL}${endpoint}`;
    const response = await fetch(url, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    if (!response.body) {
      throw new Error("Stream not supported");
    }

    return response.body.getReader();
  }
}

// Create and export API client instance
export const api = new APIClient();

// Convenience methods for common patterns
export async function withErrorHandling(
  apiCall,
  errorMessage = "Operation failed",
) {
  try {
    const result = await apiCall();
    return result;
  } catch (error) {
    console.error("withErrorHandling: API Error:", error);
    showError(error.message || errorMessage);
    throw error;
  }
}

export async function withLoading(apiCall, loadingMessage = "Loading...") {
  const { showLoading, hideLoading } = await import("$lib/stores/app.js");

  try {
    showLoading(loadingMessage);
    return await apiCall();
  } finally {
    hideLoading();
  }
}

// API endpoints
export const endpoints = {
  // Authentication
  auth: {
    login: "/api/auth/login",
    logout: "/api/auth/logout",
    register: "/api/auth/register",
    refresh: "/api/auth/refresh",
    profile: "/api/auth/profile",
  },

  // Chats
  chats: {
    list: "/api/chats",
    create: "/api/chats",
    get: (id) => `/api/chats/${id}`,
    update: (id) => `/api/chats/${id}`,
    delete: (id) => `/api/chats/${id}`,
    messages: (id) => `/api/chats/${id}/messages`,
  },

  // AI/LLM
  llm: {
    chat: "/api/llm/chat",
    stream: "/api/llm/stream",
    models: "/api/llm/models",
    usage: "/api/llm/usage",
  },

  // Settings
  settings: {
    get: "/api/settings",
    update: "/api/settings",
    apiKeys: "/api/settings/api-keys",
    prompts: "/api/settings/prompts",
  },

  // WebSocket
  ws: {
    connect: "/ws",
  },
};

export default api;
