import { writable, derived } from "svelte/store";
import { browser } from "$app/environment";
import { modelsAPI, modelUtils } from "$lib/api/models.js";
import { apiKeys } from "./settings.js";

export const userModels = writable([]);

export const availableModels = writable({});

export const enabledModels = derived(userModels, ($userModels) => {
  return $userModels.filter((model) => model.is_enabled);
});

export const enabledModelsByProvider = derived(
  enabledModels,
  ($enabledModels) => {
    return modelUtils.groupByProvider($enabledModels);
  },
);

export const modelsLoading = writable(false);
export const modelsError = writable(null);

function createLastUsedModel() {
  const { subscribe, set, update } = writable(null);

  return {
    subscribe,
    set: (value) => {
      if (browser) {
        localStorage.setItem("neko-last-used-model", JSON.stringify(value));
      }
      set(value);
    },
    update,
    init: () => {
      if (browser) {
        const stored = localStorage.getItem("neko-last-used-model");
        if (stored) {
          try {
            set(JSON.parse(stored));
          } catch (e) {
            console.warn("Failed to parse stored last used model:", e);
          }
        }
      }
    },
  };
}

export const lastUsedModel = createLastUsedModel();

export async function loadEnabledModels() {
  if (!browser) return;

  try {
    const models = await modelsAPI.getEnabledModels();
    userModels.set(models);

    return models;
  } catch (error) {
    console.error("Failed to load enabled models:", error);
    modelsError.set(error.message);
    throw error;
  }
}

export function getModelDisplayName(model) {
  if (typeof model === "string") {
    return model;
  }
  return model.model_name || model.name || model.model_id || model.id;
}

export function getProviderDisplayName(provider) {
  const names = {
    openai: "OpenAI",
    anthropic: "Anthropic",
    openrouter: "OpenRouter",
    google: "Google",
    gemini: "Google Gemini",
    xai: "xAI",
  };
  return names[provider] || provider;
}

export function providerSupportsWebSearch(provider) {
  const webSearchProviders = ["anthropic", "gemini", "openrouter"];
  return webSearchProviders.includes(provider);
}

export function anySelectedModelSupportsWebSearch(selectedModels, allModels) {
  if (!selectedModels || selectedModels.length === 0) return false;

  for (const modelId of selectedModels) {
    const model = allModels.find((m) => m.model_id === modelId);
    if (model && providerSupportsWebSearch(model.provider)) {
      return true;
    }
  }
  return false;
}

export function findModel(provider, modelId) {
  let models;
  userModels.subscribe((m) => (models = m))();

  return models.find(
    (model) => model.provider === provider && model.model_id === modelId,
  );
}

export function isModelEnabled(provider, modelId) {
  const model = findModel(provider, modelId);
  return model?.is_enabled || false;
}

export function getFirstEnabledModel() {
  let models;
  enabledModels.subscribe((m) => (models = m))();

  if (models && models.length > 0) {
    return {
      provider: models[0].provider,
      model: models[0].model_id,
    };
  }

  return {
    provider: "openai",
    model: "gpt-4o",
  };
}

export function initializeModels() {
  if (!browser) return;

  loadEnabledModels().catch((error) => {
    console.warn("Failed to load models on initialization:", error);
    userModels.set([]);
  });
}

