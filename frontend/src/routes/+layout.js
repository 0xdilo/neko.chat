import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';
import { initializeStores } from '$lib/stores/index.js';
import { refreshAuth } from '$lib/stores/auth.js';

const protectedRoutes = ['/', '/settings'];

export const load = async ({ url }) => {
  // Initialize stores immediately (non-blocking)
  initializeStores();
  
  // Start auth refresh in background - don't await to avoid blocking page load
  refreshAuth().catch(error => {
    console.warn('Background auth refresh failed:', error);
  });

  return {
    url: url.pathname
  };
};