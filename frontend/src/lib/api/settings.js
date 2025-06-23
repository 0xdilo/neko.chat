import { api, endpoints, withErrorHandling } from "./client.js";

// Settings API service
export const settingsAPI = {
  // Get all user settings
  async getSettings() {
    return withErrorHandling(
      () => api.get(endpoints.settings.get),
      "Failed to load settings.",
    );
  },

  // Add new API key
  async addApiKey(keyData) {
    return withErrorHandling(
      () =>
        api.post(endpoints.settings.apiKeys, {
          name: keyData.name,
          provider: keyData.provider,
          key: keyData.key,
          isDefault: keyData.isDefault || false,
        }),
      "Failed to add API key.",
    );
  },

  // Update API key
  async updateApiKey(keyId, updates) {
    return withErrorHandling(
      () => api.patch(`${endpoints.settings.apiKeys}/${keyId}`, updates),
      "Failed to update API key.",
    );
  },

  // Delete API key
  async deleteApiKey(keyId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.settings.apiKeys}/${keyId}`),
      "Failed to delete API key.",
    );
  },

  // Test API key validity
  async testApiKey(keyId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.apiKeys}/${keyId}/test`),
      "Failed to test API key.",
    );
  },

  // Get system prompts
  async getSystemPrompts() {
    return withErrorHandling(
      () => api.get(endpoints.settings.prompts),
      "Failed to load system prompts.",
    );
  },

  // Add system prompt
  async addSystemPrompt(promptData) {
    return withErrorHandling(
      () =>
        api.post(endpoints.settings.prompts, {
          name: promptData.name,
          prompt: promptData.prompt,
          description: promptData.description,
          isDefault: promptData.isDefault || false,
          category: promptData.category || "general",
        }),
      "Failed to add system prompt.",
    );
  },

  // Update system prompt
  async updateSystemPrompt(promptId, updates) {
    return withErrorHandling(
      () => api.patch(`${endpoints.settings.prompts}/${promptId}`, updates),
      "Failed to update system prompt.",
    );
  },

  // Delete system prompt
  async deleteSystemPrompt(promptId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.settings.prompts}/${promptId}`),
      "Failed to delete system prompt.",
    );
  },

  // Set active system prompt
  async setActivePrompt(promptId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.prompts}/${promptId}/activate`),
      "Failed to set active prompt.",
    );
  },

  // Toggle active system prompt
  async toggleActivePrompt(promptId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.prompts}/${promptId}/toggle`),
      "Failed to toggle active prompt.",
    );
  },

  // Get keyboard shortcuts
  async getKeyboardShortcuts() {
    return withErrorHandling(
      () => api.get("/api/settings/shortcuts"),
      "Failed to load keyboard shortcuts.",
    );
  },

  // Update keyboard shortcuts
  async updateKeyboardShortcuts(shortcuts) {
    return withErrorHandling(
      () => api.put("/api/settings/shortcuts", shortcuts),
      "Failed to update keyboard shortcuts.",
    );
  },
};

