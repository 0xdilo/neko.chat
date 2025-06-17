import { api, endpoints, withErrorHandling } from './client.js';

// Settings API service
export const settingsAPI = {
  // Get all user settings
  async getSettings() {
    return withErrorHandling(
      () => api.get(endpoints.settings.get),
      'Failed to load settings.'
    );
  },

  // Update user settings
  async updateSettings(settings) {
    return withErrorHandling(
      () => api.put(endpoints.settings.update, settings),
      'Failed to update settings.'
    );
  },

  // Get API keys
  async getApiKeys() {
    return withErrorHandling(
      () => api.get(endpoints.settings.apiKeys),
      'Failed to load API keys.'
    );
  },

  // Add new API key
  async addApiKey(keyData) {
    return withErrorHandling(
      () => api.post(endpoints.settings.apiKeys, {
        name: keyData.name,
        provider: keyData.provider,
        key: keyData.key,
        isDefault: keyData.isDefault || false
      }),
      'Failed to add API key.'
    );
  },

  // Update API key
  async updateApiKey(keyId, updates) {
    return withErrorHandling(
      () => api.patch(`${endpoints.settings.apiKeys}/${keyId}`, updates),
      'Failed to update API key.'
    );
  },

  // Delete API key
  async deleteApiKey(keyId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.settings.apiKeys}/${keyId}`),
      'Failed to delete API key.'
    );
  },

  // Test API key validity
  async testApiKey(keyId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.apiKeys}/${keyId}/test`),
      'Failed to test API key.'
    );
  },

  // Get system prompts
  async getSystemPrompts() {
    return withErrorHandling(
      () => api.get(endpoints.settings.prompts),
      'Failed to load system prompts.'
    );
  },

  // Add system prompt
  async addSystemPrompt(promptData) {
    return withErrorHandling(
      () => api.post(endpoints.settings.prompts, {
        name: promptData.name,
        prompt: promptData.prompt,
        description: promptData.description,
        isDefault: promptData.isDefault || false,
        category: promptData.category || 'general'
      }),
      'Failed to add system prompt.'
    );
  },

  // Update system prompt
  async updateSystemPrompt(promptId, updates) {
    return withErrorHandling(
      () => api.patch(`${endpoints.settings.prompts}/${promptId}`, updates),
      'Failed to update system prompt.'
    );
  },

  // Delete system prompt
  async deleteSystemPrompt(promptId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.settings.prompts}/${promptId}`),
      'Failed to delete system prompt.'
    );
  },

  // Set active system prompt
  async setActivePrompt(promptId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.prompts}/${promptId}/activate`),
      'Failed to set active prompt.'
    );
  },

  // Toggle active system prompt
  async toggleActivePrompt(promptId) {
    return withErrorHandling(
      () => api.post(`${endpoints.settings.prompts}/${promptId}/toggle`),
      'Failed to toggle active prompt.'
    );
  },

  // Get keyboard shortcuts
  async getKeyboardShortcuts() {
    return withErrorHandling(
      () => api.get('/api/settings/shortcuts'),
      'Failed to load keyboard shortcuts.'
    );
  },

  // Update keyboard shortcuts
  async updateKeyboardShortcuts(shortcuts) {
    return withErrorHandling(
      () => api.put('/api/settings/shortcuts', shortcuts),
      'Failed to update keyboard shortcuts.'
    );
  },

  // Reset shortcuts to default
  async resetKeyboardShortcuts() {
    return withErrorHandling(
      () => api.post('/api/settings/shortcuts/reset'),
      'Failed to reset keyboard shortcuts.'
    );
  },

  // Get user preferences
  async getPreferences() {
    return withErrorHandling(
      () => api.get('/api/settings/preferences'),
      'Failed to load preferences.'
    );
  },

  // Update preferences
  async updatePreferences(preferences) {
    return withErrorHandling(
      () => api.put('/api/settings/preferences', preferences),
      'Failed to update preferences.'
    );
  },

  // Export settings
  async exportSettings(format = 'json') {
    return withErrorHandling(
      () => api.get('/api/settings/export', { format }),
      'Failed to export settings.'
    );
  },

  // Import settings
  async importSettings(settingsData, options = {}) {
    return withErrorHandling(
      () => api.post('/api/settings/import', {
        data: settingsData,
        overwrite: options.overwrite || false,
        merge: options.merge || true
      }),
      'Failed to import settings.'
    );
  },

  // Reset all settings to default
  async resetSettings() {
    return withErrorHandling(
      () => api.post('/api/settings/reset'),
      'Failed to reset settings.'
    );
  },

  // Get settings backup
  async getSettingsBackup() {
    return withErrorHandling(
      () => api.get('/api/settings/backup'),
      'Failed to get settings backup.'
    );
  },

  // Restore from backup
  async restoreFromBackup(backupId) {
    return withErrorHandling(
      () => api.post(`/api/settings/backup/${backupId}/restore`),
      'Failed to restore from backup.'
    );
  },

  // Get usage statistics
  async getUsageStats(period = '30d') {
    return withErrorHandling(
      () => api.get('/api/settings/usage', { period }),
      'Failed to load usage statistics.'
    );
  },

  // Update notification settings
  async updateNotificationSettings(notifications) {
    return withErrorHandling(
      () => api.put('/api/settings/notifications', notifications),
      'Failed to update notification settings.'
    );
  },

  // Get privacy settings
  async getPrivacySettings() {
    return withErrorHandling(
      () => api.get('/api/settings/privacy'),
      'Failed to load privacy settings.'
    );
  },

  // Update privacy settings
  async updatePrivacySettings(privacy) {
    return withErrorHandling(
      () => api.put('/api/settings/privacy', privacy),
      'Failed to update privacy settings.'
    );
  },

  // Delete all user data
  async deleteAllData(confirmationPassword) {
    return withErrorHandling(
      () => api.post('/api/settings/delete-data', { 
        password: confirmationPassword 
      }),
      'Failed to delete user data.'
    );
  },

  // Get data export
  async requestDataExport(format = 'json') {
    return withErrorHandling(
      () => api.post('/api/settings/export-data', { format }),
      'Failed to request data export.'
    );
  },

  // Check export status
  async getExportStatus(exportId) {
    return withErrorHandling(
      () => api.get(`/api/settings/export/${exportId}/status`),
      'Failed to check export status.'
    );
  },

  // Download export
  async downloadExport(exportId) {
    return withErrorHandling(
      () => api.get(`/api/settings/export/${exportId}/download`),
      'Failed to download export.'
    );
  },
};