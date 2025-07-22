<script>
	import { Send, Square, GitFork, Globe } from 'lucide-svelte';
	import { createEventDispatcher, onMount } from 'svelte';

	export let value = '';
	export let disabled = false;
	export let isLoading = false;
	export let webSearchEnabled = false;
	export let webSearchSupported = false;
	export let rightSidebarCollapsed = true;
	export let placeholder = 'Type your message...';

	const dispatch = createEventDispatcher();

	let textarea;

	function autoResize() {
		if (textarea) {
			textarea.style.height = 'auto';
			textarea.style.height = Math.min(textarea.scrollHeight, 200) + 'px';
		}
	}

	function handleKeyDown(event) {
		if (event.key === 'Enter') {
			if (event.shiftKey) {
				// Allow new line
				return;
			} else {
				event.preventDefault();
				handleSend();
			}
		}
	}

	function handleSend() {
		if (value.trim() && !disabled && !isLoading) {
			dispatch('send', { content: value.trim() });
			value = '';
			autoResize();
		}
	}

	function handleStop() {
		dispatch('stop');
	}

	function handleToggleWebSearch() {
		dispatch('toggleWebSearch');
	}

	function handleToggleSidebar() {
		dispatch('toggleSidebar');
	}

	onMount(() => {
		autoResize();
	});

	$: if (value !== undefined) {
		autoResize();
	}
</script>

<div class="message-input-container">
	<div class="input-wrapper">
		<!-- Sidebar toggle button -->
		<button
			class="input-action-button"
			class:active={!rightSidebarCollapsed}
			on:click={handleToggleSidebar}
			title="Toggle conversation branches"
		>
			<GitFork size={18} />
		</button>

		<!-- Web search toggle button -->
		{#if webSearchSupported}
			<button
				class="input-action-button"
				class:active={webSearchEnabled}
				on:click={handleToggleWebSearch}
				title="Toggle web search"
			>
				<Globe size={18} />
			</button>
		{/if}

		<!-- Text input area -->
		<div class="textarea-container">
			<textarea
				bind:this={textarea}
				bind:value
				on:input={autoResize}
				on:keydown={handleKeyDown}
				{placeholder}
				{disabled}
				rows="1"
			></textarea>
		</div>

		<!-- Send/Stop button -->
		<button
			class="send-button"
			class:loading={isLoading}
			on:click={isLoading ? handleStop : handleSend}
			disabled={disabled || (!value.trim() && !isLoading)}
			title={isLoading ? 'Stop generation' : 'Send message'}
		>
			{#if isLoading}
				<Square size={18} />
			{:else}
				<Send size={18} />
			{/if}
		</button>
	</div>
</div>

<style>
	.message-input-container {
		position: sticky;
		bottom: 0;
		background: var(--bg-primary);
		border-top: 1px solid var(--border-color);
		padding: 1rem;
		z-index: 10;
	}

	.input-wrapper {
		display: flex;
		align-items: end;
		gap: 0.5rem;
		max-width: 800px;
		margin: 0 auto;
	}

	.input-action-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border: 1px solid var(--border-color);
		background: var(--bg-secondary);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s ease;
		color: var(--text-secondary);
		flex-shrink: 0;
	}

	.input-action-button:hover {
		background: var(--bg-tertiary);
		color: var(--text-primary);
	}

	.input-action-button.active {
		background: var(--accent-color);
		color: white;
		border-color: var(--accent-color);
	}

	.textarea-container {
		flex: 1;
		position: relative;
	}

	textarea {
		width: 100%;
		min-height: 40px;
		max-height: 200px;
		padding: 10px 12px;
		border: 1px solid var(--border-color);
		border-radius: 8px;
		background: var(--bg-secondary);
		color: var(--text-primary);
		font-family: inherit;
		font-size: 14px;
		line-height: 1.5;
		resize: none;
		outline: none;
		transition: border-color 0.2s ease;
	}

	textarea:focus {
		border-color: var(--accent-color);
	}

	textarea:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.send-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s ease;
		flex-shrink: 0;
	}

	.send-button:not(:disabled):not(.loading) {
		background: var(--accent-color);
		color: white;
	}

	.send-button.loading {
		background: #dc2626;
		color: white;
	}

	.send-button:disabled {
		background: var(--bg-tertiary);
		color: var(--text-disabled);
		cursor: not-allowed;
	}

	.send-button:not(:disabled):hover {
		transform: translateY(-1px);
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
	}
</style>