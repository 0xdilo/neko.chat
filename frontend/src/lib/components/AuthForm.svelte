<script>
  import { Eye, EyeOff, LogIn, UserPlus } from 'lucide-svelte';
  import { goto } from '$app/navigation';
  import { login, signup, isAuthenticated } from '$lib/stores/auth.js';
  import { showError, showSuccess } from '$lib/stores/app.js';
  import { authAPI } from '$lib/api/auth.js';
  import ThemeToggle from './ThemeToggle.svelte';

  export let mode = 'login';

  let email = '';
  let password = '';
  let confirmPassword = '';
  let name = '';
  let showPassword = false;
  let showConfirmPassword = false;
  let isLoading = false;
  let errors = {};

  // pre-fill admin credentials for testing
  if (mode === 'login') {
    email = 'admin@admin.com';
    password = 'admin';
  }

  function toggleMode() {
    mode = mode === 'login' ? 'signup' : 'login';
    errors = {};
  }

  function togglePasswordVisibility() {
    showPassword = !showPassword;
  }

  function toggleConfirmPasswordVisibility() {
    showConfirmPassword = !showConfirmPassword;
  }

  function validateForm() {
    errors = {};

    if (!email) {
      errors.email = 'email is required';
    } else if (!/\S+@\S+\.\S+/.test(email)) {
      errors.email = 'please enter a valid email';
    }

    if (!password) {
      errors.password = 'password is required';
    } else if (password.length < 4) {
      errors.password = 'password must be at least 4 characters';
    }

    if (mode === 'signup') {
      if (!name) {
        errors.name = 'name is required';
      }
      if (!confirmPassword) {
        errors.confirmPassword = 'please confirm your password';
      } else if (password !== confirmPassword) {
        errors.confirmPassword = 'passwords do not match';
      }
    }

    return Object.keys(errors).length === 0;
  }

  async function handleSubmit(event) {
    event.preventDefault();

    if (!validateForm()) return;

    isLoading = true;
    errors = {};

    try {
      let response;

      if (mode === 'login') {
        response = await login(email, password);
      } else {
        response = await signup(email, password, name);
      }


      if (response.success) {
        showSuccess(mode === 'login' ? 'logged in successfully!' : 'account created successfully!');
        goto('/');
      } else {
        errors.general = response.error || 'authentication failed';
      }
    } catch (error) {
      console.error('auth error:', error);
      errors.general = error.message || 'something went wrong. please try again.';
    } finally {
      isLoading = false;
    }
  }

  async function handleGoogleAuth() {
    try {
      const response = await authAPI.getGoogleAuthUrl();
      if (response.auth_url) {
        window.location.href = response.auth_url;
      }
    } catch (error) {
      showError('failed to start google authentication');
    }
  }
</script>

<div class="auth-form-container">
  <!-- moved theme toggle to the very top, within the flex container -->
  <div class="theme-toggle-container-top-right">
    <ThemeToggle />
  </div>

  <div class="form-header">
    <h1 class="form-title">
      {mode === 'login' ? 'Sign In' : 'Create Account'}
    </h1>
    <p class="form-subtitle">
      {mode === 'login'
        ? 'welcome back to neko.chat'
        : 'join the conversation'}
    </p>
  </div>

  <form on:submit={handleSubmit} class="auth-form" novalidate>


    {#if mode === 'signup'}
      <div class="form-group">
        <label for="name" class="form-label">Name</label>
        <input
          id="name"
          type="text"
          bind:value={name}
          class="form-input"
          class:error={errors.name}
          placeholder="enter your name"
          autocomplete="name"
          disabled={isLoading}
        />
        {#if errors.name}
          <span class="error-message">{errors.name}</span>
        {/if}
      </div>
    {/if}

    <div class="form-group">
      <label for="email" class="form-label">Email</label>
      <input
        id="email"
        type="email"
        bind:value={email}
        class="form-input"
        class:error={errors.email}
        placeholder="enter your email"
        autocomplete="email"
        disabled={isLoading}
      />
      {#if errors.email}
        <span class="error-message">{errors.email}</span>
      {/if}
    </div>

    <div class="form-group">
      <label for="password" class="form-label">Password</label>
      <div class="password-input">
        <input
          id="password"
          type={showPassword ? 'text' : 'password'}
          bind:value={password}
          class="form-input"
          class:error={errors.password}
          placeholder="enter your password"
          autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
          disabled={isLoading}
        />
        <button
          type="button"
          on:click={togglePasswordVisibility}
          class="password-toggle"
          aria-label={showPassword ? 'hide password' : 'show password'}
          disabled={isLoading}
        >
          {#if showPassword}
            <EyeOff size={16} />
          {:else}
            <Eye size={16} />
          {/if}
        </button>
      </div>
      {#if errors.password}
        <span class="error-message">{errors.password}</span>
      {/if}
    </div>

    {#if mode === 'signup'}
      <div class="form-group">
        <label for="confirm-password" class="form-label"
          >Confirm Password</label
        >
        <div class="password-input">
          <input
            id="confirm-password"
            type={showConfirmPassword ? 'text' : 'password'}
            bind:value={confirmPassword}
            class="form-input"
            class:error={errors.confirmPassword}
            placeholder="confirm your password"
            autocomplete="new-password"
            disabled={isLoading}
          />
          <button
            type="button"
            on:click={toggleConfirmPasswordVisibility}
            class="password-toggle"
            aria-label={showConfirmPassword
              ? 'hide password'
              : 'show password'}
            disabled={isLoading}
          >
            {#if showConfirmPassword}
              <EyeOff size={16} />
            {:else}
              <Eye size={16} />
            {/if}
          </button>
        </div>
        {#if errors.confirmPassword}
          <span class="error-message">{errors.confirmPassword}</span>
        {/if}
      </div>
    {/if}

    <button type="submit" class="submit-button" disabled={isLoading}>
      {#if isLoading}
        <div class="loading-spinner"></div>
        <span>{mode === 'login' ? 'signing in...' : 'creating account...'}</span>
      {:else}
        {#if mode === 'login'}
          <LogIn size={18} />
        {:else}
          <UserPlus size={18} />
        {/if}
        <span>{mode === 'login' ? 'sign in' : 'create account'}</span>
      {/if}
    </button>

    <div class="divider">
      <span>or</span>
    </div>

    <button
      type="button"
      on:click={handleGoogleAuth}
      class="google-button"
      disabled={isLoading}
    >
      <svg width="18" height="18" viewBox="0 0 24 24">
        <path
          fill="#4285F4"
          d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
        />
        <path
          fill="#34A853"
          d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
        />
        <path
          fill="#FBBC05"
          d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
        />
        <path
          fill="#EA4335"
          d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
        />
      </svg>
      continue with google
    </button>
  </form>

  <div class="form-footer">
    <p class="toggle-text">
      {mode === 'login' ? "don't have an account?" : 'already have an account?'}
      <button
        type="button"
        on:click={toggleMode}
        class="toggle-button"
        disabled={isLoading}
      >
        {mode === 'login' ? 'sign up' : 'sign in'}
      </button>
    </p>
  </div>
    {#if errors.general}
      <div class="error-message general">{errors.general}</div>
    {/if}
</div>

<style>
  .auth-form-container {
    display: flex;
    flex-direction: column;
    /* align-items: center; /* not needed if only pushing one item right */
    justify-content: center;
    height: 100%;
    max-width: 400px;
    margin: 0 auto;
    padding: var(--spacing-xl);
    /* removed position: relative, will assume parent or body handles overall positioning */
  }

  .theme-toggle-container-top-right {
    display: flex;
    justify-content: flex-end; /* pushes content to the right */
    width: 100%; /* allows justify-content to work across the container */
    margin-bottom: var(--spacing-md); /* space between toggle and form header */
  }

  .form-header {
    text-align: center;
    margin-bottom: var(--spacing-2xl);
  }

  .form-title {
    font-size: var(--font-size-3xl);
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: var(--spacing-sm);
    font-family: var(--font-family-mono);
    letter-spacing: -0.025em;
  }

  .form-subtitle {
    color: var(--text-secondary);
    font-size: var(--font-size-base);
  }

  .auth-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-lg);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .form-label {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-family-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .form-input {
    padding: var(--spacing-md);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-md);
    background-color: var(--bg-secondary);
    color: var(--text-primary);
    font-size: var(--font-size-base);
    transition: all var(--transition-fast);
    width: 100%;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px rgba(135, 135, 135, 0.1);
  }

  .form-input.error {
    border-color: var(--status-error);
  }

  .form-input::placeholder {
    color: var(--text-muted);
  }

  .password-input {
    position: relative;
  }

  .password-toggle {
    position: absolute;
    right: var(--spacing-md);
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: var(--spacing-xs);
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .password-toggle:hover {
    color: var(--text-primary);
    background-color: var(--interactive-hover);
  }

  .error-message {
    font-size: var(--font-size-sm);
    color: var(--status-error);
    font-family: var(--font-family-mono);
  }

  .error-message.general {
    padding: var(--spacing-md);
    background-color: rgba(118, 118, 118, 0.1);
    border: 1px solid var(--status-error);
    border-radius: var(--radius-md);
    text-align: center;
  }

  .submit-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md) var(--spacing-lg);
    background-color: var(--accent-primary);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius-md);
    font-size: var(--font-size-base);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
    margin-top: var(--spacing-md);
  }

  .submit-button:hover:not(:disabled) {
    background-color: var(--text-primary);
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
  }

  .submit-button:active:not(:disabled) {
    transform: translateY(0);
  }

  .submit-button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
    transform: none;
  }

  .loading-spinner {
    width: 18px;
    height: 18px;
    border: 2px solid transparent;
    border-top: 2px solid currentColor;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .divider {
    display: flex;
    align-items: center;
    text-align: center;
    margin: var(--spacing-lg) 0;
  }

  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border-primary);
  }

  .divider span {
    padding: 0 var(--spacing-md);
    color: var(--text-tertiary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family-mono);
  }

  .google-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md) var(--spacing-lg);
    background-color: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-md);
    font-size: var(--font-size-base);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-fast);
    width: 100%;
  }

  .google-button:hover:not(:disabled) {
    background-color: var(--interactive-hover);
    border-color: var(--border-focus);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .google-button:active:not(:disabled) {
    transform: translateY(0);
  }

  .google-button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
    transform: none;
  }

  .form-footer {
    text-align: center;
    margin-top: var(--spacing-xl);
  }

  .toggle-text {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .toggle-button {
    background: none;
    border: none;
    color: var(--accent-primary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: 600;
    text-decoration: underline;
    margin-left: var(--spacing-xs);
    transition: color var(--transition-fast);
  }

  .toggle-button:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .toggle-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 640px) {
    .auth-form-container {
      padding: var(--spacing-lg);
    }
    /* might want to center the toggle on small screens or adjust as needed */
    .theme-toggle-container-top-right {
      justify-content: center;
    }
  }
</style>
