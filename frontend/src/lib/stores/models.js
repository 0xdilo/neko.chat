import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';
import { modelsAPI, modelUtils } from '$lib/api/models.js';
import { apiKeys } from './settings.js';

// User's model preferences
export const userModels = writable([]);

// Available models from all providers
export const availableModels = writable({});

// Enabled models for chat interface
export const enabledModels = derived(userModels, ($userModels) => {
  return $userModels.filter(model => model.is_enabled);
});

// Grouped enabled models by provider
export const enabledModelsByProvider = derived(enabledModels, ($enabledModels) => {
  return modelUtils.groupByProvider($enabledModels);
});

// Favorite models
export const favoriteModels = derived(userModels, ($userModels) => {
  return $userModels.filter(model => model.is_favorite);
});

// Current loading states
export const modelsLoading = writable(false);
export const modelsError = writable(null);

/**
 * Load user's model preferences
 */
export async function loadUserModels() {
  if (!browser) return;
  
  try {
    modelsLoading.set(true);
    modelsError.set(null);
    
    const models = await modelsAPI.getUserModels();
    userModels.set(models);
    
    return models;
  } catch (error) {
    console.error('Failed to load user models:', error);
    modelsError.set(error.message);
    throw error;
  } finally {
    modelsLoading.set(false);
  }
}

/**
 * Load enabled models for chat interface
 */
export async function loadEnabledModels() {
  if (!browser) return;
  
  try {
    const models = await modelsAPI.getEnabledModels();
    // Set the user models to the enabled models from the API
    userModels.set(models);
    
    return models;
  } catch (error) {
    console.error('Failed to load enabled models:', error);
    modelsError.set(error.message);
    throw error;
  }
}

/**
 * Refresh models from all providers
 */
export async function refreshModelsFromProviders() {
  if (!browser) return;
  
  let providers;
  apiKeys.subscribe(keys => {
    providers = keys.map(key => key.provider);
  })();
  
  if (!providers || providers.length === 0) {
    throw new Error('No API keys configured');
  }
  
  try {
    modelsLoading.set(true);
    modelsError.set(null);
    
    const models = await modelsAPI.refreshAllModels(providers);
    availableModels.set(models);
    
    return models;
  } catch (error) {
    console.error('Failed to refresh models:', error);
    modelsError.set(error.message);
    throw error;
  } finally {
    modelsLoading.set(false);
  }
}

/**
 * Update user model preferences
 */
export async function updateModelPreferences(models) {
  if (!browser) return;
  
  try {
    modelsLoading.set(true);
    modelsError.set(null);
    
    await modelsAPI.updateUserModelPreferences(models);
    userModels.set(models);
    
    return models;
  } catch (error) {
    console.error('Failed to update model preferences:', error);
    modelsError.set(error.message);
    throw error;
  } finally {
    modelsLoading.set(false);
  }
}

/**
 * Get display name for a model
 */
export function getModelDisplayName(model) {
  if (typeof model === 'string') {
    return model;
  }
  return model.model_name || model.name || model.model_id || model.id;
}

/**
 * Get provider display name
 */
export function getProviderDisplayName(provider) {
  const names = {
    openai: 'OpenAI',
    anthropic: 'Anthropic',
    openrouter: 'OpenRouter',
    google: 'Google',
    gemini: 'Google Gemini',
    xai: 'xAI'
  };
  return names[provider] || provider;
}

/**
 * Find a model by provider and ID
 */
export function findModel(provider, modelId) {
  let models;
  userModels.subscribe(m => models = m)();
  
  return models.find(model => 
    model.provider === provider && model.model_id === modelId
  );
}

/**
 * Check if a model is enabled
 */
export function isModelEnabled(provider, modelId) {
  const model = findModel(provider, modelId);
  return model?.is_enabled || false;
}

/**
 * Check if a model is favorited
 */
export function isModelFavorite(provider, modelId) {
  const model = findModel(provider, modelId);
  return model?.is_favorite || false;
}

/**
 * Get default models when no preferences are set
 */
export function getDefaultEnabledModels() {
  return [
    {
      provider: 'openai',
      model_id: 'gpt-4o',
      model_name: 'GPT-4o',
      is_enabled: true,
      is_favorite: false,
      display_order: 0
    },
    {
      provider: 'openai', 
      model_id: 'gpt-4o-mini',
      model_name: 'GPT-4o Mini',
      is_enabled: true,
      is_favorite: false,
      display_order: 1
    }
  ];
}

/**
 * Get the first enabled model (fallback to default)
 */
export function getFirstEnabledModel() {
  let models;
  enabledModels.subscribe(m => models = m)();
  
  if (models && models.length > 0) {
    return {
      provider: models[0].provider,
      model: models[0].model_id
    };
  }
  
  // Fallback to OpenAI GPT-4o if no enabled models
  return {
    provider: 'openai',
    model: 'gpt-4o'
  };
}

/**
 * Initialize models after authentication is confirmed
 */
export function initializeModels() {
  if (!browser) return;
  
  // Load user models when authentication is confirmed
  loadEnabledModels().catch(error => {
    console.warn('Failed to load models on initialization:', error);
    // Set default models if loading fails
    userModels.set(getDefaultEnabledModels());
  });
}