<script>
  import '../app.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { theme } from '$lib/theme.js';
  import { initializeStores, initializeModelsAfterAuth } from '$lib/stores/index.js';
  import { refreshAuth, isAuthenticated } from '$lib/stores/auth.js';
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
    initializeStores();
    
    await refreshAuth();
    authChecked = true;
    
    if ($isAuthenticated) {
      initializeModelsAfterAuth().then(() => {
        modelsInitialized = true;
      });
      autoConnectWebSocket();
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

