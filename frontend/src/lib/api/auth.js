import { api, endpoints, withErrorHandling } from "./client.js";

export const authAPI = {
  async login(email, password) {
    return withErrorHandling(
      () => api.post(endpoints.auth.login, { email, password }),
      "Login failed. Please check your credentials.",
    );
  },

  async register(email, password, name) {
    return withErrorHandling(
      () => api.post(endpoints.auth.register, { email, password, name }),
      "Registration failed. Please try again.",
    );
  },

  async logout() {
    return withErrorHandling(
      () => api.post(endpoints.auth.logout),
      "Logout failed.",
    );
  },

  async getProfile() {
    return withErrorHandling(
      () => api.get(endpoints.auth.profile),
      "Failed to load profile.",
    );
  },

  async getGoogleAuthUrl() {
    return withErrorHandling(
      () => api.get("/api/auth/google"),
      "Failed to get Google auth URL.",
    );
  },

  async googleCallback(code, state) {
    return withErrorHandling(
      () => api.get(`/api/auth/google/callback?code=${code}&state=${state}`),
      "Google authentication failed.",
    );
  },
};
