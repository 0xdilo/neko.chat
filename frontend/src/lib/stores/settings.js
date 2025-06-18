import { writable } from "svelte/store";
import { browser } from "$app/environment";
import { keysAPI } from "../api/keys.js";

// Default settings
const defaultSettings = {
  // Profile
  userName: "",

  // API Keys
  apiKeys: [],

  // System Prompts
  systemPrompts: [
    {
      id: "default",
      name: "Default Assistant",
      prompt: "You are a helpful AI assistant.",
      active: true,
    },
  ],

  // Appearance
  theme: "dark",
  fontSize: "medium",
  messageAnimation: true,
  compactMode: false,

  // Behavior
  autoSave: true,
  soundNotifications: false,
  showTypingIndicator: true,

  // Keyboard shortcuts
  keybindings: {
    enabled: true,
    vimMode: true,
    customShortcuts: {},
  },

  // Chat settings
  defaultModel: "gpt-4",
  defaultTemperature: 0.7,
  defaultMaxTokens: 2048,

  // Privacy
  saveConversations: true,
  analytics: false,

  // Advanced
  debugMode: false,
  experimentalFeatures: false,
};

// Settings store
export const settings = writable(defaultSettings);

// Individual setting stores for easier component binding
export const userName = writable(defaultSettings.userName);
export const apiKeys = writable(defaultSettings.apiKeys);
export const apiKeysLoaded = writable(false);
export const systemPrompts = writable(defaultSettings.systemPrompts);
export const appearance = writable({
  theme: defaultSettings.theme,
  fontSize: defaultSettings.fontSize,
  messageAnimation: defaultSettings.messageAnimation,
  compactMode: defaultSettings.compactMode,
});

// Update a specific setting
export function updateSetting(key, value) {
  settings.update((current) => {
    const updated = { ...current, [key]: value };

    if (browser) {
      localStorage.setItem("neko-settings", JSON.stringify(updated));
    }

    return updated;
  });

  // Also update the individual store
  if (key === "apiKeys") {
    apiKeys.set(value);
  }
}

// Update nested setting
export function updateNestedSetting(path, value) {
  settings.update((current) => {
    const updated = { ...current };
    const keys = path.split(".");
    let target = updated;

    for (let i = 0; i < keys.length - 1; i++) {
      target = target[keys[i]];
    }

    target[keys[keys.length - 1]] = value;

    if (browser) {
      localStorage.setItem("neko-settings", JSON.stringify(updated));
    }

    return updated;
  });
}

// API Key management
export function addApiKey(keyData) {
  const newKey = {
    id: Date.now(),
    ...keyData,
    createdAt: new Date(),
    masked: true,
  };

  apiKeys.update((keys) => {
    const updated = [...keys, newKey];
    updateSetting("apiKeys", updated);
    return updated;
  });

  return newKey;
}

export function updateApiKey(keyId, updates) {
  apiKeys.update((keys) => {
    const updated = keys.map((key) =>
      key.id === keyId ? { ...key, ...updates } : key,
    );
    updateSetting("apiKeys", updated);
    return updated;
  });
}

export function removeApiKey(keyId) {
  apiKeys.update((keys) => {
    const updated = keys.filter((key) => key.id !== keyId);
    updateSetting("apiKeys", updated);
    return updated;
  });
}

export function toggleApiKeyVisibility(keyId) {
  apiKeys.update((keys) => {
    const updated = keys.map((key) =>
      key.id === keyId ? { ...key, masked: !key.masked } : key,
    );
    updateSetting("apiKeys", updated);
    return updated;
  });
}

// System Prompt management
export function addSystemPrompt(promptData) {
  const newPrompt = {
    id: Date.now(),
    ...promptData,
    active: false,
    createdAt: new Date(),
  };

  systemPrompts.update((prompts) => {
    const updated = [...prompts, newPrompt];
    updateSetting("systemPrompts", updated);
    return updated;
  });

  return newPrompt;
}

export function updateSystemPrompt(promptId, updates) {
  systemPrompts.update((prompts) => {
    const updated = prompts.map((prompt) =>
      prompt.id === promptId ? { ...prompt, ...updates } : prompt,
    );
    updateSetting("systemPrompts", updated);
    return updated;
  });
}

export function removeSystemPrompt(promptId) {
  systemPrompts.update((prompts) => {
    const updated = prompts.filter((prompt) => prompt.id !== promptId);
    updateSetting("systemPrompts", updated);
    return updated;
  });
}

export function setActiveSystemPrompt(promptId) {
  systemPrompts.update((prompts) => {
    const updated = prompts.map((prompt) => ({
      ...prompt,
      active: prompt.id === promptId,
    }));
    updateSetting("systemPrompts", updated);
    return updated;
  });
}

// Get active system prompt
export function getActiveSystemPrompt() {
  let activePrompt;
  systemPrompts.subscribe((prompts) => {
    activePrompt = prompts.find((prompt) => prompt.active);
  })();
  return activePrompt || defaultSettings.systemPrompts[0];
}

// User name settings
export function updateUserName(name) {
  userName.set(name);
  updateSetting("userName", name);
}

// Appearance settings
export function updateAppearance(updates) {
  appearance.update((current) => {
    const updated = { ...current, ...updates };

    // Update individual settings
    Object.entries(updates).forEach(([key, value]) => {
      updateSetting(key, value);

      // Sync theme changes with the theme store
      if (key === "theme" && typeof window !== "undefined") {
        import("../theme.js").then(({ theme }) => {
          theme.set(value);
        });
      }
    });

    return updated;
  });
}

// Reset settings to default
export function resetSettings() {
  settings.set(defaultSettings);
  userName.set(defaultSettings.userName);
  apiKeys.set(defaultSettings.apiKeys);
  apiKeysLoaded.set(false);
  systemPrompts.set(defaultSettings.systemPrompts);
  appearance.set({
    theme: defaultSettings.theme,
    fontSize: defaultSettings.fontSize,
    messageAnimation: defaultSettings.messageAnimation,
    compactMode: defaultSettings.compactMode,
  });

  if (browser) {
    localStorage.setItem("neko-settings", JSON.stringify(defaultSettings));
  }
}

// Load API keys from backend
export async function loadApiKeysFromBackend() {
  if (!browser) {
    apiKeysLoaded.set(true);
    return;
  }

  // Add a safety timeout to prevent infinite loading
  const timeoutId = setTimeout(() => {
    console.warn(
      "loadApiKeysFromBackend: TIMEOUT - Force setting apiKeysLoaded to true after 5 seconds",
    );
    apiKeysLoaded.set(true);
  }, 5000);

  try {
    const response = await keysAPI.getKeys();

    // Backend returns direct array, not wrapped in response object
    const keysArray = Array.isArray(response) ? response : [];

    if (keysArray.length > 0) {
      const formattedKeys = keysArray.map((key) => ({
        id: key.id || Date.now(),
        provider: key.provider,
        name: key.name || key.provider,
        key: key.key || "***",
        masked: true,
        createdAt: key.created_at ? new Date(key.created_at) : new Date(),
      }));

      apiKeys.set(formattedKeys);
      updateSetting("apiKeys", formattedKeys);
    } else {
      // Still set empty array to ensure store is updated
      apiKeys.set([]);
      updateSetting("apiKeys", []);
    }
  } catch (error) {
    console.error(
      "loadApiKeysFromBackend: FAILED to load API keys from backend:",
      error,
    );
    console.error("loadApiKeysFromBackend: Error details:", {
      message: error.message,
      stack: error.stack,
      name: error.name,
    });
    // Set empty array on error too
    apiKeys.set([]);
    updateSetting("apiKeys", []);
  } finally {
    // Clear the timeout since we're completing normally
    clearTimeout(timeoutId);
    // Mark API keys as loaded regardless of success/failure
    apiKeysLoaded.set(true);
  }
}

// Initialize settings from localStorage
export function initializeSettings() {
  if (!browser) return;

  try {
    const stored = localStorage.getItem("neko-settings");
    if (stored) {
      const loadedSettings = JSON.parse(stored);

      // Merge with defaults to ensure all properties exist
      const mergedSettings = { ...defaultSettings, ...loadedSettings };

      settings.set(mergedSettings);
      userName.set(mergedSettings.userName || "");
      apiKeys.set(mergedSettings.apiKeys || []);
      systemPrompts.set(
        mergedSettings.systemPrompts || defaultSettings.systemPrompts,
      );
      appearance.set({
        theme: mergedSettings.theme,
        fontSize: mergedSettings.fontSize,
        messageAnimation: mergedSettings.messageAnimation,
        compactMode: mergedSettings.compactMode,
      });

      // If we have API keys in localStorage, mark as loaded initially
      // The backend loading will update them if needed
      if (mergedSettings.apiKeys && mergedSettings.apiKeys.length > 0) {
        apiKeysLoaded.set(true);
      } else {
        apiKeysLoaded.set(false);
      }
    } else {
      // No stored settings, so we need to load from backend
      apiKeysLoaded.set(false);
    }
  } catch (error) {
    console.error("Failed to load settings from localStorage:", error);
    apiKeysLoaded.set(false);
    resetSettings();
  }
}

// Export settings
export function exportSettings() {
  let settingsData;
  settings.subscribe((data) => (settingsData = data))();

  const exportData = {
    settings: settingsData,
    exportDate: new Date().toISOString(),
    version: "1.0",
  };

  return JSON.stringify(exportData, null, 2);
}

// Import settings
export function importSettings(jsonData) {
  try {
    const importData = JSON.parse(jsonData);

    if (importData.settings) {
      const mergedSettings = { ...defaultSettings, ...importData.settings };

      settings.set(mergedSettings);
      userName.set(mergedSettings.userName || "");
      apiKeys.set(mergedSettings.apiKeys || []);
      systemPrompts.set(
        mergedSettings.systemPrompts || defaultSettings.systemPrompts,
      );
      appearance.set({
        theme: mergedSettings.theme,
        fontSize: mergedSettings.fontSize,
        messageAnimation: mergedSettings.messageAnimation,
        compactMode: mergedSettings.compactMode,
      });

      if (browser) {
        localStorage.setItem("neko-settings", JSON.stringify(mergedSettings));
      }

      return true;
    }

    throw new Error("Invalid settings data format");
  } catch (error) {
    console.error("Failed to import settings:", error);
    return false;
  }
}

