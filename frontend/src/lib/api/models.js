import { api } from './client.js';

/**
 * API client for model management
 */
export const modelsAPI = {
  /**
   * Fetch available models from a provider
   * @param {string} provider - The provider name (openai, anthropic, openrouter)
   * @returns {Promise<Array>} List of available models
   */
  async fetchModelsForProvider(provider) {
    return api.post('/api/settings/models/fetch', { provider });
  },

  /**
   * Get user's saved model preferences
   * @returns {Promise<Array>} List of user model preferences
   */
  async getUserModels() {
    return api.get('/api/settings/models');
  },

  /**
   * Update user's model preferences
   * @param {Array} models - Array of model preference objects
   * @returns {Promise<Object>} Success response
   */
  async updateUserModelPreferences(models) {
    return api.put('/api/settings/models', { models });
  },

  /**
   * Get only enabled models for the user
   * @returns {Promise<Array>} List of enabled models
   */
  async getEnabledModels() {
    return api.get('/api/settings/models/enabled');
  },

  /**
   * Toggle a model's enabled state
   * @param {string} provider - The provider name
   * @param {string} modelId - The model ID
   * @param {string} modelName - The model display name
   * @returns {Promise<Object>} Toggle result with new state
   */
  async toggleModel(provider, modelId, modelName) {
    return api.post('/api/settings/models/toggle', {
      provider,
      model_id: modelId,
      model_name: modelName
    });
  },

  /**
   * Refresh models for all providers that have API keys
   * @param {Array} providers - List of providers to refresh
   * @returns {Promise<Object>} Map of provider -> models
   */
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
  }
};

/**
 * Model utility functions
 */
export const modelUtils = {
  /**
   * Filter models based on search query
   * @param {Array} models - List of models
   * @param {string} query - Search query
   * @returns {Array} Filtered models
   */
  filterModels(models, query) {
    if (!query) return models;
    
    const lowerQuery = query.toLowerCase();
    return models.filter(model => 
      model.id.toLowerCase().includes(lowerQuery) ||
      model.name.toLowerCase().includes(lowerQuery) ||
      (model.description && model.description.toLowerCase().includes(lowerQuery))
    );
  },

  /**
   * Group models by provider
   * @param {Array} models - List of models
   * @returns {Object} Models grouped by provider
   */
  groupByProvider(models) {
    return models.reduce((acc, model) => {
      if (!acc[model.provider]) {
        acc[model.provider] = [];
      }
      acc[model.provider].push(model);
      return acc;
    }, {});
  },

  /**
   * Sort models by various criteria
   * @param {Array} models - List of models
   * @param {string} sortBy - Sort criteria (name, created, provider)
   * @param {string} order - Sort order (asc, desc)
   * @returns {Array} Sorted models
   */
  sortModels(models, sortBy = 'name', order = 'asc') {
    const sorted = [...models].sort((a, b) => {
      let aVal, bVal;
      
      switch (sortBy) {
        case 'name':
          aVal = a.name.toLowerCase();
          bVal = b.name.toLowerCase();
          break;
        case 'created':
          aVal = a.created;
          bVal = b.created;
          break;
        case 'provider':
          aVal = a.provider.toLowerCase();
          bVal = b.provider.toLowerCase();
          break;
        default:
          aVal = a.name.toLowerCase();
          bVal = b.name.toLowerCase();
      }
      
      if (aVal < bVal) return order === 'asc' ? -1 : 1;
      if (aVal > bVal) return order === 'asc' ? 1 : -1;
      return 0;
    });
    
    return sorted;
  },

  /**
   * Get popular/recommended models for each provider
   * @param {Object} modelsByProvider - Models grouped by provider
   * @returns {Object} Popular models by provider
   */
  getPopularModels(modelsByProvider) {
    const popular = {
      openai: ['gpt-4o', 'gpt-4o-mini', 'gpt-3.5-turbo'],
      anthropic: ['claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022', 'claude-3-opus-20240229'],
      openrouter: ['openai/gpt-4o', 'anthropic/claude-3-5-sonnet', 'google/gemini-pro']
    };
    
    const result = {};
    
    Object.entries(modelsByProvider).forEach(([provider, models]) => {
      const popularIds = popular[provider] || [];
      result[provider] = models.filter(model => 
        popularIds.some(id => model.id.includes(id))
      );
    });
    
    return result;
  },

  /**
   * Create default model preferences from fetched models
   * @param {Array} models - Raw models from API
   * @param {Array} popularModels - List of popular model IDs
   * @returns {Array} Default model preferences
   */
  createDefaultPreferences(models, popularModels = []) {
    return models.map((model, index) => ({
      provider: model.provider,
      model_id: model.id,
      model_name: model.name,
      is_enabled: popularModels.includes(model.id),
      is_favorite: false,
      display_order: index
    }));
  }
};

export default modelsAPI;