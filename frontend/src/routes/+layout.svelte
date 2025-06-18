<script>
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { theme } from '$lib/theme.js';
  import { initializeStores, initializeModelsAfterAuth } from '$lib/stores/index.js';
  import { refreshAuth, isAuthenticated } from '$lib/stores/auth.js';
  import { apiKeysLoaded } from '$lib/stores/settings.js';
  import KeybindingProvider from '$lib/components/KeybindingProvider.svelte';

  let authChecked = false;
  let showContent = false;
  let modelsInitialized = false;

  // Protected routes that require authentication
  const protectedRoutes = ['/', '/settings'];
  
  $: currentPath = $page.url.pathname;
  $: isProtectedRoute = protectedRoutes.includes(currentPath);
  
  // Handle showing content when route changes after auth is checked
  $: if (authChecked && !isProtectedRoute) {
    showContent = true;
  }
  
  $: if (authChecked && !$isAuthenticated && isProtectedRoute) {
    goto('/auth');
  }

  $: if (authChecked && $isAuthenticated && currentPath === '/auth') {
    goto('/');
  }

  // Initialize models and API keys when user becomes authenticated
  $: if (authChecked && $isAuthenticated && !modelsInitialized) {
    initializeModelsAfterAuth().then(() => {
      modelsInitialized = true;
    });
  }


  onMount(async () => {
    theme.init();
    initializeStores();
    
    // Check authentication status
    
    const authResult = await refreshAuth();
    authChecked = true;
    
    // Initialize models and load API keys only if authenticated
    if ($isAuthenticated) {
      await initializeModelsAfterAuth();
      modelsInitialized = true;
    }
    
    // Handle route display logic after auth check
    if (isProtectedRoute) {
      if ($isAuthenticated) {
        showContent = true;
      } else {
        goto('/auth');
        // Don't set showContent here - let the redirect happen and onMount will run again
      }
    } else {
      // Non-protected route (like /auth)
      showContent = true;
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
