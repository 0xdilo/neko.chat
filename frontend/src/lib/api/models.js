import { api } from "./client.js";

export const modelsAPI = {
  async fetchModelsForProvider(provider) {
    return api.post("/api/settings/models/fetch", { provider });
  },

  async getUserModels() {
    return api.get("/api/settings/models");
  },

  async getEnabledModels() {
    return api.get("/api/settings/models/enabled");
  },

  async toggleModel(provider, modelId, modelName) {
    return api.post("/api/settings/models/toggle", {
      provider,
      model_id: modelId,
      model_name: modelName,
    });
  },

  async refreshAllModels(providers) {
    const results = {};

    for (const provider of providers) {
      try {
        results[provider] = await this.fetchModelsForProvider(provider);
      } catch (error) {
        console.error(`Failed to fetch models for ${provider}:`, error);
        results[provider] = [];
      }
    }

    return results;
  },
};

export const modelUtils = {
  filterModels(models, query) {
    if (!query) return models;

    const lowerQuery = query.toLowerCase();
    return models.filter(
      (model) =>
        model.id.toLowerCase().includes(lowerQuery) ||
        model.name.toLowerCase().includes(lowerQuery) ||
        (model.description &&
          model.description.toLowerCase().includes(lowerQuery)),
    );
  },

  groupByProvider(models) {
    return models.reduce((acc, model) => {
      if (!acc[model.provider]) {
        acc[model.provider] = [];
      }
      acc[model.provider].push(model);
      return acc;
    }, {});
  },

  sortModels(models, sortBy = "name", order = "asc") {
    const sorted = [...models].sort((a, b) => {
      let aVal, bVal;

      switch (sortBy) {
        case "name":
          aVal = a.name.toLowerCase();
          bVal = b.name.toLowerCase();
          break;
        case "created":
          aVal = a.created;
          bVal = b.created;
          break;
        case "provider":
          aVal = a.provider.toLowerCase();
          bVal = b.provider.toLowerCase();
          break;
        default:
          aVal = a.name.toLowerCase();
          bVal = b.name.toLowerCase();
      }

      if (aVal < bVal) return order === "asc" ? -1 : 1;
      if (aVal > bVal) return order === "asc" ? 1 : -1;
      return 0;
    });

    return sorted;
  },
};

export default modelsAPI;

