// Export all stores for convenient importing
export * from "./chats.js";
export * from "./models.js";
export * from "./settings.js";
export * from "./auth.js";
export * from "./app.js";

// Re-export theme store
export { theme, themes } from "../theme.js";

// Initialize all stores
import { initializeChats } from "./chats.js";
import { initializeModels } from "./models.js";
import { initializeSettings, loadApiKeysFromBackend, apiKeysLoaded } from "./settings.js";
import { initializeAuth } from "./auth.js";
import { initializeApp } from "./app.js";

export function initializeStores() {
  initializeChats();
  initializeSettings();
  initializeAuth();
  initializeApp();
}

export async function initializeModelsAfterAuth() {
  console.log('initializeModelsAfterAuth: Starting initialization...');
  
  console.log('initializeModelsAfterAuth: Calling initializeModels()...');
  initializeModels();
  
  // Add a small delay to ensure auth token is properly established
  console.log('initializeModelsAfterAuth: Waiting 100ms for auth to stabilize...');
  await new Promise(resolve => setTimeout(resolve, 100));
  
  console.log('initializeModelsAfterAuth: Calling loadApiKeysFromBackend()...');
  // Load API keys from backend after authentication
  await loadApiKeysFromBackend();
  console.log('initializeModelsAfterAuth: Initialization complete');
}

