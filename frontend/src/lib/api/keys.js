import { api, withErrorHandling } from "./client.js";

export const keysAPI = {
  async getKeys() {
    try {
      const result = await api.request("/api/keys", { noRedirectOn401: true });
      return result;
    } catch (error) {
      console.error("keysAPI.getKeys: Error occurred:", error);
      if (
        error.message.includes("Authentication") ||
        error.message.includes("401")
      ) {
        console.warn("keysAPI.getKeys: Auth error, returning empty array");
        return [];
      }
      console.warn(
        "keysAPI.getKeys: Non-auth error, returning empty array:",
        error.message,
      );
      return [];
    }
  },

  async addKey(provider, apiKey) {
    return withErrorHandling(
      () =>
        api.post("/api/keys", {
          provider,
          api_key: apiKey,
        }),
      "Failed to add API key.",
    );
  },

  async getKey(provider) {
    return withErrorHandling(
      () => api.get(`/api/keys/${provider}`),
      "Failed to get API key.",
    );
  },

  async deleteKey(provider) {
    return withErrorHandling(
      () => api.delete(`/api/keys/${provider}`),
      "Failed to delete API key.",
    );
  },
};
