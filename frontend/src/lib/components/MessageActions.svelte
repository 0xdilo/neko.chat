<script>
	import { Copy, Edit, Trash2, GitFork, RotateCcw } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';

	export let messageType; // 'user' | 'bot'
	export let messageId;
	export let messageContent;
	export let canRetry = false;

	const dispatch = createEventDispatcher();

	function handleCopy() {
		dispatch('copy', { content: messageContent });
	}

	function handleEdit() {
		dispatch('edit', { messageId, content: messageContent });
	}

	function handleDelete() {
		dispatch('delete', { messageId });
	}

	function handleFork() {
		dispatch('fork', { messageId });
	}

	function handleRetry() {
		dispatch('retry', { messageId });
	}
</script>

<div class="message-actions">
	<button
		class="action-button"
		on:click={handleCopy}
		title="Copy message"
	>
		<Copy size={16} />
	</button>

	{#if messageType === 'bot'}
		{#if canRetry}
			<button
				class="action-button"
				on:click={handleRetry}
				title="Retry generation"
			>
				<RotateCcw size={16} />
			</button>
		{/if}
		<button
			class="action-button"
			on:click={handleFork}
			title="Fork conversation"
		>
			<GitFork size={16} />
		</button>
	{:else}
		<button
			class="action-button"
			on:click={handleEdit}
			title="Edit message"
		>
			<Edit size={16} />
		</button>
	{/if}

	<button
		class="action-button delete-button"
		on:click={handleDelete}
		title="Delete message"
	>
		<Trash2 size={16} />
	</button>
</div>

<style>
	.message-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.action-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid var(--border-color);
		background: var(--bg-secondary);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s ease;
		color: var(--text-secondary);
	}

	.action-button:hover {
		background: var(--bg-tertiary);
		color: var(--text-primary);
		transform: translateY(-1px);
	}

	.delete-button:hover {
		background: #dc2626;
		color: white;
		border-color: #dc2626;
	}
</style>