import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { authAPI } from '$lib/api/auth.js';

// Authentication state
export const isAuthenticated = writable(false);
export const user = writable(null);
export const isLoading = writable(false);
export const authError = writable(null);

// Combined auth store for easy access
export const auth = writable(null);

// User data structure
const defaultUser = {
  id: null,
  email: '',
  name: '',
  avatar: null,
  plan: 'free', // free, pro, enterprise
  usage: {
    messagesThisMonth: 0,
    tokensUsed: 0,
    limit: 100
  },
  preferences: {
    emailNotifications: true,
    securityAlerts: true
  },
  createdAt: null,
  lastLoginAt: null
};

// Authentication actions
export async function login(email, password) {
  console.log('Login attempt for:', email);
  isLoading.set(true);
  authError.set(null);
  
  try {
    console.log('Calling authAPI.login...');
    const response = await authAPI.login(email, password);
    console.log('Login response:', response);
    
    if (response.token && response.user) {
      // Store token
      if (browser) {
        localStorage.setItem('neko-auth-token', response.token);
        console.log('Token stored in localStorage');
      }
      
      // Update stores
      user.set(response.user);
      auth.set(response.user);
      isAuthenticated.set(true);
      console.log('Authentication state updated');
      
      return { success: true, user: response.user };
    } else {
      throw new Error('Invalid response from server');
    }
  } catch (error) {
    console.error('Login error:', error);
    const errorMessage = error.message || 'Login failed';
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

export async function signup(email, password, name = '') {
  isLoading.set(true);
  authError.set(null);
  
  try {
    const response = await authAPI.register(email, password, name || email.split('@')[0]);
    
    if (response) {
      // Registration successful, now login
      return await login(email, password);
    } else {
      throw new Error('Registration failed');
    }
  } catch (error) {
    const errorMessage = error.message || 'Signup failed';
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

export async function logout() {
  isLoading.set(true);
  
  try {
    // Clear local state
    user.set(null);
    auth.set(null);
    isAuthenticated.set(false);
    authError.set(null);
    
    if (browser) {
      localStorage.removeItem('neko-auth-token');
    }
    
    return { success: true };
  } catch (error) {
    console.error('Logout error:', error);
    return { success: false, error: error.message };
  } finally {
    isLoading.set(false);
  }
}

export async function refreshAuth() {
  if (!browser) {
    console.log('Not in browser, skipping auth refresh');
    return { success: false };
  }
  
  const token = localStorage.getItem('neko-auth-token');
  console.log('Token from localStorage:', token ? 'exists' : 'missing');
  
  if (token) {
    try {
      console.log('Attempting to get profile with token...');
      const userData = await authAPI.getProfile();
      console.log('Profile data received:', userData);
      user.set(userData);
      auth.set(userData);
      isAuthenticated.set(true);
      return { success: true };
    } catch (error) {
      console.error('Failed to refresh auth:', error);
      await logout();
      return { success: false };
    }
  }
  
  console.log('No token found, user not authenticated');
  isAuthenticated.set(false);
  return { success: false };
}

// User profile actions
export async function updateProfile(updates) {
  isLoading.set(true);
  
  try {
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 800));
    
    user.update(current => {
      if (!current) return current;
      
      const updated = { ...current, ...updates };
      
      if (browser) {
        localStorage.setItem('neko-user', JSON.stringify(updated));
      }
      
      return updated;
    });
    
    return { success: true };
  } catch (error) {
    authError.set(error.message);
    return { success: false, error: error.message };
  } finally {
    isLoading.set(false);
  }
}

export async function changePassword(currentPassword, newPassword) {
  isLoading.set(true);
  authError.set(null);
  
  try {
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Mock password change
    console.log('Password changed successfully');
    
    return { success: true };
  } catch (error) {
    const errorMessage = error.message || 'Failed to change password';
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

export async function deleteAccount() {
  isLoading.set(true);
  
  try {
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1500));
    
    // Clear all data
    await logout();
    
    return { success: true };
  } catch (error) {
    authError.set(error.message);
    return { success: false, error: error.message };
  } finally {
    isLoading.set(false);
  }
}

// Password reset
export async function requestPasswordReset(email) {
  isLoading.set(true);
  authError.set(null);
  
  try {
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 800));
    
    console.log('Password reset email sent to:', email);
    
    return { success: true };
  } catch (error) {
    const errorMessage = error.message || 'Failed to send reset email';
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

export async function resetPassword(token, newPassword) {
  isLoading.set(true);
  authError.set(null);
  
  try {
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    console.log('Password reset successfully');
    
    return { success: true };
  } catch (error) {
    const errorMessage = error.message || 'Failed to reset password';
    authError.set(errorMessage);
    return { success: false, error: errorMessage };
  } finally {
    isLoading.set(false);
  }
}

// Initialize authentication state
export function initializeAuth() {
  if (!browser) return;
  
  refreshAuth();
}

// Clear authentication error
export function clearAuthError() {
  authError.set(null);
}

// Utility functions
export function isLoggedIn() {
  let authenticated = false;
  isAuthenticated.subscribe(value => authenticated = value)();
  return authenticated;
}

export function getCurrentUser() {
  let currentUser = null;
  user.subscribe(value => currentUser = value)();
  return currentUser;
}

export function hasValidToken() {
  if (!browser) return false;
  return !!localStorage.getItem('neko-auth-token');
}