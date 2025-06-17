import { api, withErrorHandling } from './client.js';

// API Keys API service
export const keysAPI = {
  // Get all API keys for the current user
  async getKeys() {
    console.log('keysAPI.getKeys: Starting API call...');
    try {
      // Use direct API call with no redirect on 401 to handle auth gracefully during initial load
      const result = await api.request('/api/keys', { noRedirectOn401: true });
      console.log('keysAPI.getKeys: Direct API result:', result);
      return result;
    } catch (error) {
      console.error('keysAPI.getKeys: Error occurred:', error);
      // If it's an auth error, return empty array instead of failing
      if (error.message.includes('Authentication') || error.message.includes('401')) {
        console.warn('keysAPI.getKeys: Auth error, returning empty array');
        return [];
      }
      // For other errors, still return empty array to prevent infinite loading
      console.warn('keysAPI.getKeys: Non-auth error, returning empty array:', error.message);
      return [];
    }
  },

  // Add a new API key
  async addKey(provider, apiKey) {
    return withErrorHandling(
      () => api.post('/api/keys', {
        provider,
        api_key: apiKey
      }),
      'Failed to add API key.'
    );
  },

  // Get a specific API key (decrypted)
  async getKey(provider) {
    return withErrorHandling(
      () => api.get(`/api/keys/${provider}`),
      'Failed to get API key.'
    );
  },

  // Delete an API key
  async deleteKey(provider) {
    return withErrorHandling(
      () => api.delete(`/api/keys/${provider}`),
      'Failed to delete API key.'
    );
  },

  // Test an API key (if we want to add validation)
  async testKey(provider, apiKey) {
    return withErrorHandling(
      () => api.post(`/api/keys/test`, {
        provider,
        api_key: apiKey
      }),
      'Failed to test API key.'
    );
  }
};