import { api, endpoints, withErrorHandling } from "./client.js";

export const settingsAPI = {
  async getSystemPrompts() {
    return withErrorHandling(
      () => api.get(endpoints.settings.prompts),
      "Failed to load system prompts.",
    );
  },

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

  async updateSystemPrompt(promptId, updates) {
    return withErrorHandling(
      () => api.patch(`${endpoints.settings.prompts}/${promptId}`, updates),
      "Failed to update system prompt.",
    );
  },

  async deleteSystemPrompt(promptId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.settings.prompts}/${promptId}`),
      "Failed to delete system prompt.",
    );
  },

  async toggleActivePrompt(promptId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.prompts}/${promptId}/toggle`),
      "Failed to toggle active prompt.",
    );
  },
};

