// Export all stores for convenient importing
export * from "./chats.js";
export * from "./models.js";
export * from "./settings.js";
export * from "./auth.js";
export * from "./app.js";

export { theme, themes } from "../theme.js";

import { initializeChats } from "./chats.js";
import { initializeModels } from "./models.js";
import {
  initializeSettings,
  loadApiKeysFromBackend,
  apiKeysLoaded,
} from "./settings.js";
import { initializeAuth } from "./auth.js";

export function initializeStores() {
  initializeChats();
  initializeSettings();
  initializeAuth();
}

export async function initializeModelsAfterAuth() {
  initializeModels();
  await new Promise((resolve) => setTimeout(resolve, 100));
  await loadApiKeysFromBackend();
}
