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
    console.log('Non-protected route after auth check, showing content');
    showContent = true;
  }
  
  $: if (authChecked && !$isAuthenticated && isProtectedRoute) {
    console.log('Redirecting to auth - not authenticated');
    goto('/auth');
  }

  $: if (authChecked && $isAuthenticated && currentPath === '/auth') {
    console.log('Redirecting to home - authenticated');
    goto('/');
  }

  // Initialize models and API keys when user becomes authenticated
  $: if (authChecked && $isAuthenticated && !modelsInitialized) {
    console.log('User authenticated, initializing models and API keys...');
    initializeModelsAfterAuth().then(() => {
      console.log('Models and API keys initialization complete');
      modelsInitialized = true;
    });
  }


  onMount(async () => {
    theme.init();
    initializeStores();
    
    // Check authentication status
    console.log('Checking auth status...');
    console.log('Current path:', currentPath);
    console.log('Is protected route:', isProtectedRoute);
    
    const authResult = await refreshAuth();
    console.log('Auth result:', authResult);
    console.log('Is authenticated:', $isAuthenticated);
    authChecked = true;
    
    // Initialize models and load API keys only if authenticated
    if ($isAuthenticated) {
      console.log('Already authenticated on mount, initializing models and API keys...');
      await initializeModelsAfterAuth();
      modelsInitialized = true;
    }
    
    // Handle route display logic after auth check
    if (isProtectedRoute) {
      if ($isAuthenticated) {
        console.log('Authenticated, showing protected content');
        showContent = true;
      } else {
        console.log('Not authenticated, redirecting to auth');
        goto('/auth');
        // Don't set showContent here - let the redirect happen and onMount will run again
      }
    } else {
      // Non-protected route (like /auth)
      console.log('Non-protected route, showing content');
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
