import { writable } from "svelte/store";
import { browser } from "$app/environment";
import { authAPI } from "$lib/api/auth.js";

export const isAuthenticated = writable(false);
export const user = writable(null);
export const isLoading = writable(false);
export const authError = writable(null);

export const auth = writable(null);

export async function login(email, password) {
  isLoading.set(true);
  authError.set(null);

  try {
    const response = await authAPI.login(email, password);
    if (response.token && response.user) {
      if (browser) {
        localStorage.setItem("neko-auth-token", response.token);
      }

      user.set(response.user);
      auth.set(response.user);
      isAuthenticated.set(true);

      return { success: true, user: response.user };
    } else {
      throw new Error("Invalid response from server");
    }
  } catch (error) {
    console.error("Login error:", error);
    const errorMessage = error.message || "Login failed";
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

export async function signup(email, password, name = "") {
  isLoading.set(true);
  authError.set(null);

  try {
    const response = await authAPI.register(
      email,
      password,
      name || email.split("@")[0],
    );

    if (response) {
      return await login(email, password);
    } else {
      throw new Error("Registration failed");
    }
  } catch (error) {
    const errorMessage = error.message || "Signup failed";
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

export async function logout() {
  isLoading.set(true);

  try {
    user.set(null);
    auth.set(null);
    isAuthenticated.set(false);
    authError.set(null);

    if (browser) {
      localStorage.removeItem("neko-auth-token");
    }

    return { success: true };
  } catch (error) {
    console.error("Logout error:", error);
    return { success: false, error: error.message };
  } finally {
    isLoading.set(false);
  }
}

export async function refreshAuth() {
  if (!browser) {
    return { success: false };
  }

  const token = localStorage.getItem("neko-auth-token");

  if (token) {
    try {
      const userData = await authAPI.getProfile();
      user.set(userData);
      auth.set(userData);
      isAuthenticated.set(true);
      return { success: true };
    } catch (error) {
      console.error("Failed to refresh auth:", error);
      await logout();
      return { success: false };
    }
  }

  isAuthenticated.set(false);
  return { success: false };
}

export function initializeAuth() {
  if (!browser) return;

  refreshAuth();
}
