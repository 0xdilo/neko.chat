<script>
	import { createEventDispatcher, onMount } from "svelte";
	import { Eye, EyeOff, Trash2, Plus, X, Check } from "lucide-svelte";
	import { fade, scale } from "svelte/transition";
	import { keysAPI } from '$lib/api/keys.js';
	import { showError, showSuccess } from '$lib/stores/app.js';
	import { apiKeys as apiKeysStore, updateSetting } from '$lib/stores/settings.js';

	export let apiKeys = [];
	const dispatch = createEventDispatcher();

	let showAddModal = false;
	let newApiKey = { provider: "openai", key: "" };
	let deletingProvider = null;
	let isLoading = false;
	let visibleKeys = new Set();
	let decryptedKeys = new Map();

	onMount(async () => {
		await loadApiKeys();
	});

	async function loadApiKeys() {
		isLoading = true;
		try {
			const response = await keysAPI.getKeys();
			const keysArray = Array.isArray(response) ? response : [];
			
			const formattedKeys = keysArray.map(key => ({
				provider: key.provider,
				created_at: key.created_at,
				masked: true
			}));
			
			apiKeys = formattedKeys;
			updateSetting('apiKeys', formattedKeys);
			
		} catch (error) {
			console.error('Failed to load API keys:', error);
			showError('Failed to load API keys');
		} finally {
			isLoading = false;
		}
	}

	async function toggleVisibility(provider) {
		if (visibleKeys.has(provider)) {
			visibleKeys.delete(provider);
			decryptedKeys.delete(provider);
		} else {
			try {
				const keyData = await keysAPI.getKey(provider);
				decryptedKeys.set(provider, keyData.api_key);
				visibleKeys.add(provider);
			} catch (error) {
				console.error('Failed to decrypt API key:', error);
				showError('Failed to decrypt API key');
				return;
			}
		}
		visibleKeys = visibleKeys;
		decryptedKeys = decryptedKeys; 
	}

	async function addKey() {
		if (!newApiKey.key.trim()) {
			showError('Please enter an API key');
			return;
		}

		isLoading = true;
		try {
			await keysAPI.addKey(newApiKey.provider, newApiKey.key);
			showSuccess('API key added successfully');
			showAddModal = false;
			
			const newKey = {
				provider: newApiKey.provider,
				created_at: new Date().toISOString(),
				masked: true
			};
			const updatedKeys = [...apiKeys, newKey];
			updateSetting('apiKeys', updatedKeys);
			
			newApiKey = { provider: "openai", key: "" };
			
			setTimeout(() => loadApiKeys(), 100);
		} catch (error) {
			console.error('Failed to add API key:', error);
			showError('Failed to add API key');
		} finally {
			isLoading = false;
		}
	}

	function requestRemove(provider) {
		deletingProvider = provider;
	}

	async function confirmRemove(provider) {
		isLoading = true;
		try {
			await keysAPI.deleteKey(provider);
			showSuccess('API key deleted successfully');
			deletingProvider = null;
			
			const updatedKeys = apiKeys.filter(key => key.provider !== provider);
			updateSetting('apiKeys', updatedKeys);
			
			setTimeout(() => loadApiKeys(), 100);
		} catch (error) {
			console.error('Failed to delete API key:', error);
			showError('Failed to delete API key');
		} finally {
			isLoading = false;
		}
	}
</script>

<div class="section-header">
	<div>
		<h2 class="section-title">api keys</h2>
		<p class="section-description">
			manage your api keys for different ai providers.
		</p>
	</div>
	<button class="add-btn" on:click={() => (showAddModal = true)}>
		<Plus size={16} />
		<span>add key</span>
	</button>
</div>

<div class="list-container">
	{#if isLoading}
		<div class="loading-state">
			<div class="loading-spinner"></div>
			<p>Loading API keys...</p>
		</div>
	{:else}
		{#each apiKeys as apiKey (apiKey.provider)}
			<div class="list-item">
				<div class="item-details">
					<span class="item-name">{apiKey.provider}</span>
					<span class="item-created">Added: {new Date(apiKey.created_at).toLocaleDateString()}</span>
				</div>
				<div class="item-actions">
					<code class="api-key-display">
						{visibleKeys.has(apiKey.provider)
							? decryptedKeys.get(apiKey.provider) || "Loading..."
							: "••••••••••••••••"}
					</code>
					<button
						class="action-btn"
						on:click={() => toggleVisibility(apiKey.provider)}
						disabled={isLoading}
					>
						{#if visibleKeys.has(apiKey.provider)} <EyeOff size={16} /> {:else} <Eye size={16} /> {/if}
					</button>
					{#if deletingProvider === apiKey.provider}
						<button
							class="action-btn confirm"
							on:click={() => confirmRemove(apiKey.provider)}
							transition:scale|local={{ duration: 150 }}
							disabled={isLoading}
						>
							<Check size={16} />
						</button>
						<button
							class="action-btn"
							on:click={() => (deletingProvider = null)}
							transition:scale|local={{ duration: 150 }}
							disabled={isLoading}
						>
							<X size={16} />
						</button>
					{:else}
						<button
							class="action-btn danger"
							on:click={() => requestRemove(apiKey.provider)}
							transition:scale|local={{ duration: 150 }}
							disabled={isLoading}
						>
							<Trash2 size={16} />
						</button>
					{/if}
				</div>
			</div>
		{:else}
			<div class="empty-state">
				<p>no api keys added yet.</p>
			</div>
		{/each}
	{/if}
</div>

{#if showAddModal}
	<div
		class="modal-overlay"
		transition:fade={{ duration: 200 }}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		on:click={() => (showAddModal = false)}
		on:keydown={(e) => e.key === 'Escape' && (showAddModal = false)}
	>
		<div
			class="modal-content"
			transition:scale={{ duration: 200, start: 0.95 }}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			on:click|stopPropagation
			on:keydown|stopPropagation
		>
			<div class="modal-header">
				<h3>add new api key</h3>
				<button class="close-btn" on:click={() => (showAddModal = false)}>
					<X size={20} />
				</button>
			</div>
			<form class="modal-body">
				<div class="form-group">
					<label for="provider">provider</label>
					<select id="provider" bind:value={newApiKey.provider} disabled={isLoading}>
						<option value="openai">OpenAI</option>
						<option value="anthropic">Anthropic</option>
						<option value="openrouter">OpenRouter</option>
						<option value="xai">xAI</option>
						<option value="gemini">Google Gemini</option>
					</select>
				</div>
				<div class="form-group">
					<label for="key">api key</label>
					<input
						id="key"
						type="password"
						bind:value={newApiKey.key}
						placeholder="Enter your API key..."
						disabled={isLoading}
					/>
					<small class="help-text">
						Your API key will be securely encrypted and stored.
					</small>
				</div>
			</form>
			<div class="modal-footer">
				<button 
					class="cancel-btn" 
					on:click={() => (showAddModal = false)}
					disabled={isLoading}
				>
					cancel
				</button>
				<button 
					class="submit-btn" 
					on:click={addKey}
					disabled={isLoading || !newApiKey.key.trim()}
				>
					{#if isLoading}
						<div class="loading-spinner-small"></div>
					{/if}
					add key
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
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
	.section-description {
		color: var(--text-secondary);
		max-width: 65ch;
	}
	.add-btn {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-lg);
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		font-weight: 600;
		white-space: nowrap;
	}
	.list-container {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}
	.list-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-md) var(--spacing-lg);
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
	}
	.item-details {
		display: flex;
		flex-direction: column;
	}
	.item-name {
		font-weight: 600;
		color: var(--text-primary);
	}
	.item-created {
		font-size: var(--font-size-sm);
		color: var(--text-secondary);
	}
	.item-actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.api-key-display {
		font-family: var(--font-family-mono);
		background-color: var(--bg-tertiary);
		padding: var(--spacing-sm);
		border-radius: var(--radius-md);
		color: var(--text-secondary);
	}
	.action-btn {
		padding: var(--spacing-sm);
		border-radius: var(--radius-md);
		border: 1px solid transparent;
		color: var(--text-secondary);
	}
	.action-btn:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}
	.action-btn.danger:hover {
		background-color: var(--status-error-muted);
		color: var(--status-error);
	}
	.action-btn.confirm:hover {
		background-color: var(--status-success-muted);
		color: var(--status-success);
	}
	.empty-state {
		text-align: center;
		padding: var(--spacing-2xl);
		color: var(--text-muted);
		border: 2px dashed var(--border-primary);
		border-radius: var(--radius-lg);
	}
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(4px);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.modal-content {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		width: 100%;
		max-width: 500px;
		box-shadow: var(--shadow-lg);
	}
	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--border-primary);
	}
	.modal-header h3 {
		font-size: var(--font-size-lg);
		font-weight: 600;
	}
	.close-btn {
		color: var(--text-secondary);
	}
	.modal-body {
		padding: var(--spacing-lg);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}
	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}
	.form-group label {
		font-weight: 500;
		text-transform: capitalize;
	}
	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
		background-color: var(--bg-tertiary);
		border-top: 1px solid var(--border-primary);
		border-bottom-left-radius: var(--radius-lg);
		border-bottom-right-radius: var(--radius-lg);
	}
	.cancel-btn {
		background-color: transparent;
		border: 1px solid var(--border-primary);
	}
	.submit-btn {
		background-color: var(--accent-primary);
		color: var(--bg-primary);
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-md);
		padding: var(--spacing-2xl);
		color: var(--text-secondary);
	}

	.loading-spinner {
		width: 32px;
		height: 32px;
		border: 3px solid var(--border-primary);
		border-top: 3px solid var(--accent-primary);
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.help-text {
		font-size: var(--font-size-xs);
		color: var(--text-muted);
		margin-top: var(--spacing-xs);
	}
</style>
