<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { authAPI } from '$lib/api/auth.js';
  import { auth } from '$lib/stores/auth.js';
  import { showError, showSuccess } from '$lib/stores/app.js';

  let loading = true;
  let error = null;

  onMount(async () => {
    try {
      const urlParams = new URLSearchParams($page.url.search);
      const code = urlParams.get('code');
      const state = urlParams.get('state');

      if (!code) {
        throw new Error('No authorization code received');
      }

      const response = await authAPI.googleCallback(code, state);
      
      if (response.token) {
        localStorage.setItem('neko-auth-token', response.token);
        auth.set(response.user);
        showSuccess('Successfully signed in with Google!');
        goto('/');
      }
    } catch (err) {
      console.error('Google OAuth callback error:', err);
      error = err.message || 'Authentication failed';
      showError(error);
      
      // Redirect to auth page after a delay
      setTimeout(() => {
        goto('/auth');
      }, 3000);
    } finally {
      loading = false;
    }
  });
</script>

<div class="callback-container">
  {#if loading}
    <div class="loading">
      <div class="loading-spinner"></div>
      <p>Completing authentication...</p>
    </div>
  {:else if error}
    <div class="error">
      <h2>Authentication Failed</h2>
      <p>{error}</p>
      <p>Redirecting to login page...</p>
    </div>
  {/if}
</div>

<style>
  .callback-container {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    padding: var(--spacing-xl);
  }

  .loading,
  .error {
    text-align: center;
    max-width: 400px;
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-primary);
    border-top: 3px solid var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto var(--spacing-lg);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading p,
  .error p {
    color: var(--text-secondary);
    margin-bottom: var(--spacing-sm);
  }

  .error h2 {
    color: var(--status-error);
    margin-bottom: var(--spacing-md);
  }
</style>