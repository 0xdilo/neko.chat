<script>
	export let keyboardShortcuts = [];
</script>

<div class="section-header">
	<div>
		<h2 class="section-title">keyboard shortcuts</h2>
		<p class="section-description">
			review available shortcuts for faster navigation and actions.
		</p>
	</div>
</div>

<div class="shortcuts-list">
	{#if keyboardShortcuts.length > 0}
		{#each keyboardShortcuts as shortcut (shortcut.id)}
			<div class="shortcut-item">
				<div class="shortcut-action">{shortcut.action}</div>
				<div class="shortcut-keys">
					{#each shortcut.keys as key}
						<kbd>{key}</kbd>
					{/each}
				</div>
			</div>
		{/each}
	{:else}
		<p class="no-shortcuts-message">no shortcuts defined.</p>
	{/if}
</div>

<style>
	.section-header {
		margin-bottom: var(--spacing-xl);
		padding-bottom: var(--spacing-lg);
		border-bottom: 1px solid var(--border-primary);
	}
	.section-title {
		font-size: var(--font-size-2xl);
		font-weight: 700;
		color: var(--text-primary);
		margin-bottom: var(--spacing-sm);
		font-family: var(--font-family-mono);
	}
	.section-description,
	.no-shortcuts-message {
		color: var(--text-secondary);
	}
	.shortcuts-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}
	.shortcut-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-md);
		background-color: var(--bg-secondary);
		border-radius: var(--radius-lg);
	}
	.shortcut-action {
		font-weight: 500;
		color: var(--text-primary);
		text-transform: capitalize;
	}
	.shortcut-keys {
		display: flex;
		gap: var(--spacing-xs);
	}
	.shortcut-keys kbd {
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--text-primary);
		text-transform: capitalize;
	}
</style>
