<script>
  import { userName, apiKeys, apiKeysLoaded } from '$lib/stores/settings.js';
  import ASCIIArt from './ASCIIArt.svelte';

  export let onSendMessage = null;

  $: name = $userName;
  $: userApiKeys = $apiKeys;
  $: keysLoaded = $apiKeysLoaded;
  $: hasApiKeys = userApiKeys && userApiKeys.length > 0;
  
  const examplePrompts = [
    'Explain quantum Meow simply',
    'What is the meaning of Meow?',
    'What makes humans Meow?',
    'How does the Meow work?',
  ];

  async function handlePromptClick(prompt) {
    try {
      if (onSendMessage) {
        // Use the provided handler from ChatInterface (parallel-aware)
        await onSendMessage(prompt);
      } else {
        // Fallback to direct store method (single model only)
        const { sendMessage } = await import('$lib/stores/chats.js');
        await sendMessage(prompt);
      }
    } catch (error) {
      console.error('failed to send prompt:', error);
    }
  }
</script>

<div class="welcome-container">
  <div class="welcome-content">
    {#if !keysLoaded}
      <div class="loading-notice">
        <div class="loading-spinner"></div>
        <p>Loading...</p>
      </div>
    {:else if !hasApiKeys}
      <div class="api-key-notice">
        <div class="notice-content">
          <h3>API Key Required</h3>
          <p>To start chatting, you need to add an API key from a supported provider.</p>
          <a href="/settings?section=api-keys" class="settings-button">
            Add API Key in Settings
          </a>
        </div>
      </div>
    {:else}
      <div class="welcome-header">
        <h1 class="welcome-title">
          how can i help you{name ? `, ${name}` : ''}?
        </h1>
      </div>

      <div class="prompts-divider">
        <span class="prompts-label">try asking me about</span>
      </div>

      <div class="example-prompts">
        <div class="prompts-grid">
          {#each examplePrompts as prompt, index}
            <button
              class="prompt-button"
              on:click={() => handlePromptClick(prompt)}
              style="animation-delay: {index * 0.1}s"
            >
              {prompt}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .welcome-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    padding: var(--spacing-2xl);
    text-align: center;
  }

  .welcome-content {
    max-width: 600px;
    width: 100%;
  }

  .welcome-header {
    margin-bottom: var(--spacing-2xl);
  }

  .welcome-title {
    font-size: var(--font-size-2xl);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.3;
    animation: welcome-fade-in 0.8s ease-out;
  }

  .example-prompts {
    margin-bottom: var(--spacing-xl);
    animation: welcome-fade-in 1s ease-out 0.2s both;
  }

  .prompts-divider {
    display: flex;
    align-items: center;
    text-align: center;
    margin: var(--spacing-xl) 0;
    animation: welcome-fade-in 1s ease-out 0.2s both; /* maintain animation */
  }

  .prompts-divider::before,
  .prompts-divider::after {
    content: '';
    flex: 1;
    border-bottom: 1px solid var(--border-primary);
  }

  .prompts-divider:not(:empty)::before {
    margin-right: 0.5em;
  }

  .prompts-divider:not(:empty)::after {
    margin-left: 0.5em;
  }

  .prompts-label {
    font-size: var(--font-size-base);
    color: var(--text-secondary);
    font-weight: 500;
    white-space: nowrap; /* prevent text from wrapping */
  }

  .prompts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: var(--spacing-md);
    margin-top: var(--spacing-lg);
  }

  .prompt-button {
    background: transparent; 
    border: 0px;
    border-radius: var(--radius-lg);
    padding: var(--spacing-md) var(--spacing-lg);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
    opacity: 0;
    animation: welcome-slide-up 0.6s ease-out forwards;
  }

  .prompt-button:hover {
    background: var(--bg-tertiary);
    border-color: var(--border-secondary);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .prompt-button:active {
    transform: translateY(0);
  }

  .name-suggestion {
    padding: var(--spacing-lg);
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-lg);
    animation: welcome-fade-in 1.2s ease-out 0.4s both;
  }

  .name-suggestion p {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .suggestion-text {
    font-weight: 600;
    color: var(--accent-primary);
  }

  .settings-link {
    color: var(--accent-primary);
    text-decoration: none;
    font-weight: 500;
    transition: color var(--transition-fast);
  }

  .settings-link:hover {
    color: var(--accent-primary-dark, var(--accent-primary));
    text-decoration: underline;
  }

  .api-key-notice {
    padding: var(--spacing-xl);
    background: var(--bg-secondary);
    border: 1px solid var(--accent-primary);
    border-radius: var(--radius-lg);
    text-align: center;
    animation: welcome-fade-in 1s ease-out 0.2s both;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .notice-ascii {
    display: flex;
    justify-content: center;
    max-width: 100%;
    overflow: hidden;
    text-align: left; /* Override parent text-align */
  }

  .notice-ascii :global(.ascii-art) {
    font-size: 0.7rem;
    text-align: left !important; /* Force left alignment for ASCII art */
  }

  .notice-ascii :global(.ascii-container) {
    text-align: left !important; /* Ensure container doesn't center content */
  }

  .notice-content h3 {
    margin: 0 0 var(--spacing-md) 0;
    font-size: var(--font-size-lg);
    color: var(--text-primary);
    font-weight: 600;
  }

  .notice-content p {
    margin: 0 0 var(--spacing-lg) 0;
    color: var(--text-secondary);
    font-size: var(--font-size-base);
    line-height: 1.5;
  }

  .settings-button {
    display: inline-block;
    background: var(--accent-primary);
    color: white;
    text-decoration: none;
    padding: var(--spacing-md) var(--spacing-xl);
    border-radius: var(--radius-lg);
    font-weight: 500;
    font-size: var(--font-size-base);
    transition: all var(--transition-fast);
  }

  .settings-button:hover {
    background: var(--accent-primary-dark, var(--accent-primary));
    transform: translateY(-1px);
  }

  .loading-notice {
    padding: var(--spacing-xl);
    text-align: center;
    animation: welcome-fade-in 1s ease-out 0.2s both;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .loading-notice .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-primary);
    border-top: 3px solid var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  .loading-notice p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-base);
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  @keyframes welcome-float {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-10px);
    }
  }

  @keyframes welcome-fade-in {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes welcome-slide-up {
    from {
      opacity: 0;
      transform: translateY(30px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (max-width: 768px) {
    .welcome-container {
      padding: var(--spacing-xl) var(--spacing-lg);
      min-height: 50vh;
    }

    .welcome-title {
      font-size: var(--font-size-xl);
    }

    .prompts-grid {
      grid-template-columns: 1fr;
      gap: var(--spacing-sm);
    }

  }
</style>
