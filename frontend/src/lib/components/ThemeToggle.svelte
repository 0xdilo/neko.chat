<script>
  import { theme, themes } from '$lib/theme.js';
  import { Moon, Sun } from 'lucide-svelte';

  function toggleTheme() {
    theme.toggle();
  }
</script>

<button
  on:click={toggleTheme}
  class="theme-toggle"
  aria-label="Toggle theme"
  title="Toggle between dark and light theme"
>
  {#if $theme === themes.dark}
    <Sun size={18} />
  {:else}
    <Moon size={18} />
  {/if}
</button>

<style>
  .theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .theme-toggle:hover {
    background-color: var(--interactive-hover);
    color: var(--text-primary);
    border-color: var(--border-secondary);
  }

  .theme-toggle:active {
    background-color: var(--interactive-active);
  }
</style>
