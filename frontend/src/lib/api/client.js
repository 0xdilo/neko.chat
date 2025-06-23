import { browser } from "$app/environment";
import { showError, showWarning } from "$lib/stores/app.js";

const API_BASE_URL = browser
  ? window.location.origin.replace(":5173", ":8080")
  : "http://localhost:8080";

class APIClient {
  constructor(baseURL = API_BASE_URL) {
    this.baseURL = baseURL;
    this.defaultHeaders = {
      "Content-Type": "application/json",
    };
  }

  getAuthToken() {
    if (!browser) return null;
    return localStorage.getItem("neko-auth-token");
  }

  getHeaders(customHeaders = {}) {
    const headers = { ...this.defaultHeaders, ...customHeaders };

    const token = this.getAuthToken();
    if (token) {
      headers.Authorization = `Bearer ${token}`;
    }

    return headers;
  }

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

      switch (response.status) {
        case 401:
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

  async request(endpoint, options = {}) {
    const url = `${this.baseURL}${endpoint}`;
    const config = {
      headers: this.getHeaders(options.headers),
      ...options,
    };

    const { noRedirectOn401, ...fetchOptions } = config;

    const controller = new AbortController();
    fetchOptions.signal = controller.signal;

    this.currentRequest = { noRedirectOn401 };

    try {
      const response = await fetch(url, fetchOptions);
      const result = await this.handleResponse(response);
      return result;
    } catch (error) {
      console.error("API Request failed:", error);

      if (error.name === "AbortError") {
        throw new Error("Request timeout");
      }

      if (!navigator.onLine) {
        showWarning("You are offline. Please check your connection.");
        throw new Error("Network unavailable");
      }

      throw error;
    }
  }

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

  async upload(endpoint, formData) {
    return this.request(endpoint, {
      method: "POST",
      headers: this.getHeaders({ "Content-Type": undefined }), // Let browser set content-type for FormData
      body: formData,
    });
  }

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

export const api = new APIClient();

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

export const endpoints = {
  // Authentication
  auth: {
    login: "/api/auth/login",
    logout: "/api/auth/logout",
    register: "/api/auth/register",
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
