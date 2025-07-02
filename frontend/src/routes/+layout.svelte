<script>
  import '../app.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { theme } from '$lib/theme.js';
  import { initializeStores, initializeModelsAfterAuth } from '$lib/stores/index.js';
  import { refreshAuth, isAuthenticated } from '$lib/stores/auth.js';
  import { loadApiKeysFromBackend } from '$lib/stores/settings.js';
  import { autoConnectWebSocket } from '$lib/api/websocket.js';
  import KeybindingProvider from '$lib/components/KeybindingProvider.svelte';

  export let data;

  let modelsInitialized = false;
  let authChecked = false;

  $: if ($isAuthenticated && !modelsInitialized) {
    initializeModelsAfterAuth().then(() => {
      modelsInitialized = true;
    });
  }

  // Handle redirect from /auth to / when authenticated
  $: if ($isAuthenticated && data.currentPath === '/auth') {
    goto('/');
  }

  // Show content logic
  $: showContent = authChecked && (
    !data.isProtectedRoute || 
    $isAuthenticated || 
    data.currentPath === '/auth'
  );

  onMount(async () => {
    theme.init();
    
    // Progressive loading - start with essential features first
    try {
      // Essential: Check if user is already authenticated
      const authResult = await refreshAuth();
      authChecked = true;
      
      if (authResult && authResult.success) {
        // User is authenticated - load user-specific data in background
        Promise.all([
          loadApiKeysFromBackend(),
          initializeModelsAfterAuth()
        ]).then(() => {
          modelsInitialized = true;
          // Auto-connect WebSocket after user data is loaded
          autoConnectWebSocket();
        }).catch(error => {
          console.warn('Background initialization failed:', error);
        });
      }
    } catch (error) {
      console.warn('Auth initialization failed:', error);
      authChecked = true;
    }
  });
</script>

<KeybindingProvider>
  {#if showContent}
    <slot />
  {:else}
    <div class="loading-screen">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <p>Loading...</p>
      </div>
    </div>
  {/if}
</KeybindingProvider>

<style>
  .loading-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--bg-primary);
  }

  .loading-content {
    text-align: center;
    color: var(--text-secondary);
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-primary);
    border-top: 3px solid var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
