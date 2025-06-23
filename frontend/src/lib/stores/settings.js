import { writable } from "svelte/store";
import { browser } from "$app/environment";
import { keysAPI } from "../api/keys.js";

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

  // Keyboard shortcuts
  keybindings: {
    enabled: true,
    vimMode: true,
    customShortcuts: {},
  },
};

export const settings = writable(defaultSettings);

export const userName = writable(defaultSettings.userName);
export const apiKeys = writable(defaultSettings.apiKeys);
export const apiKeysLoaded = writable(false);
export const systemPrompts = writable(defaultSettings.systemPrompts);
export const appearance = writable({
  theme: defaultSettings.theme,
  fontSize: defaultSettings.fontSize,
  messageAnimation: defaultSettings.messageAnimation,
});

export function updateSetting(key, value) {
  settings.update((current) => {
    const updated = { ...current, [key]: value };

    if (browser) {
      localStorage.setItem("neko-settings", JSON.stringify(updated));
    }

    return updated;
  });

  if (key === "apiKeys") {
    apiKeys.set(value);
  }
}

export function updateUserName(name) {
  userName.set(name);
  updateSetting("userName", name);
}

export function updateAppearance(updates) {
  appearance.update((current) => {
    const updated = { ...current, ...updates };

    Object.entries(updates).forEach(([key, value]) => {
      updateSetting(key, value);

      if (key === "theme" && typeof window !== "undefined") {
        import("../theme.js").then(({ theme }) => {
          theme.set(value);
        });
      }
    });

    return updated;
  });
}

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
  });

  if (browser) {
    localStorage.setItem("neko-settings", JSON.stringify(defaultSettings));
  }
}

export async function loadApiKeysFromBackend() {
  if (!browser) {
    apiKeysLoaded.set(true);
    return;
  }

  const timeoutId = setTimeout(() => {
    console.warn(
      "loadApiKeysFromBackend: TIMEOUT - Force setting apiKeysLoaded to true after 5 seconds",
    );
    apiKeysLoaded.set(true);
  }, 5000);

  try {
    const response = await keysAPI.getKeys();

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
    apiKeys.set([]);
    updateSetting("apiKeys", []);
  } finally {
    clearTimeout(timeoutId);
    apiKeysLoaded.set(true);
  }
}

export function initializeSettings() {
  if (!browser) return;

  try {
    const stored = localStorage.getItem("neko-settings");
    if (stored) {
      const loadedSettings = JSON.parse(stored);

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
      });

      if (mergedSettings.apiKeys && mergedSettings.apiKeys.length > 0) {
        apiKeysLoaded.set(true);
      } else {
        apiKeysLoaded.set(false);
      }
    } else {
      apiKeysLoaded.set(false);
    }
  } catch (error) {
    console.error("Failed to load settings from localStorage:", error);
    apiKeysLoaded.set(false);
    resetSettings();
  }
}
