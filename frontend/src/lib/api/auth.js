import { api, endpoints, withErrorHandling } from './client.js';

// Authentication API service
export const authAPI = {
  // Login with email and password
  async login(email, password) {
    return withErrorHandling(
      () => api.post(endpoints.auth.login, { email, password }),
      'Login failed. Please check your credentials.'
    );
  },

  // Register new user
  async register(email, password, name) {
    return withErrorHandling(
      () => api.post(endpoints.auth.register, { email, password, name }),
      'Registration failed. Please try again.'
    );
  },

  // Logout current user
  async logout() {
    return withErrorHandling(
      () => api.post(endpoints.auth.logout),
      'Logout failed.'
    );
  },

  // Refresh authentication token
  async refresh() {
    return withErrorHandling(
      () => api.post(endpoints.auth.refresh),
      'Session refresh failed.'
    );
  },

  // Get user profile
  async getProfile() {
    return withErrorHandling(
      () => api.get(endpoints.auth.profile),
      'Failed to load profile.'
    );
  },

  // Update user profile
  async updateProfile(profileData) {
    return withErrorHandling(
      () => api.put(endpoints.auth.profile, profileData),
      'Failed to update profile.'
    );
  },

  // Change password
  async changePassword(currentPassword, newPassword) {
    return withErrorHandling(
      () => api.post(`${endpoints.auth.profile}/password`, {
        currentPassword,
        newPassword
      }),
      'Failed to change password.'
    );
  },

  // Request password reset
  async requestPasswordReset(email) {
    return withErrorHandling(
      () => api.post('/api/auth/password-reset', { email }),
      'Failed to send reset email.'
    );
  },

  // Reset password with token
  async resetPassword(token, newPassword) {
    return withErrorHandling(
      () => api.post('/api/auth/password-reset/confirm', {
        token,
        newPassword
      }),
      'Failed to reset password.'
    );
  },

  // Delete account
  async deleteAccount(password) {
    return withErrorHandling(
      () => api.delete(endpoints.auth.profile, { password }),
      'Failed to delete account.'
    );
  },

  // Google OAuth
  async getGoogleAuthUrl() {
    return withErrorHandling(
      () => api.get('/api/auth/google'),
      'Failed to get Google auth URL.'
    );
  },

  // Google OAuth callback
  async googleCallback(code, state) {
    return withErrorHandling(
      () => api.get(`/api/auth/google/callback?code=${code}&state=${state}`),
      'Google authentication failed.'
    );
  },
};