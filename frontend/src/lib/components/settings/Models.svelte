<script>
	import { Search, RefreshCw, Check, X, ChevronDown, ChevronUp } from 'lucide-svelte';
	import { onMount, createEventDispatcher } from 'svelte';
	import { modelsAPI, modelUtils } from '$lib/api/models.js';
import { loadEnabledModels } from '$lib/stores/models.js';
	import { apiKeys } from '$lib/stores/settings.js';
	import { showError, showSuccess } from '$lib/stores/app.js';

	const dispatch = createEventDispatcher();

	// Component state
	let searchQuery = '';
	let isLoading = false;
	let isRefreshing = false;
	let availableModels = {};
	let userModelPreferences = [];
	let filteredModels = {};
	let expandedProviders = {};
	let showOnlyEnabled = false;

	// Get providers with API keys
	$: providersWithKeys = $apiKeys.map(key => key.provider);

	// Filter and sort models based on search and options
	$: {
		// Start with available models
		let filtered = { ...availableModels };
		
		// Apply search filter
		if (searchQuery) {
			Object.keys(filtered).forEach(provider => {
				filtered[provider] = modelUtils.filterModels(filtered[provider], searchQuery);
			});
		}
		
		// Apply default sorting (newest to oldest)
		Object.keys(filtered).forEach(provider => {
			filtered[provider] = modelUtils.sortModels(filtered[provider], 'created', 'desc');
		});
		
		filteredModels = filtered;
	}

	// Initialize on mount
	onMount(async () => {
		await loadUserModelPreferences();
		if (providersWithKeys.length > 0) {
			await refreshModels();
		}
	});

	// Watch for API key changes
	$: if (providersWithKeys.length > 0) {
		// Auto-refresh when new API keys are added
		refreshModels();
	}

	async function loadUserModelPreferences() {
		try {
			userModelPreferences = await modelsAPI.getUserModels();
		} catch (error) {
			console.error('Failed to load user model preferences:', error);
			userModelPreferences = [];
		}
	}

	async function refreshModels() {
		if (providersWithKeys.length === 0) {
			showError('Add API keys first to fetch models');
			return;
		}

		isRefreshing = true;
		try {
			const results = await modelsAPI.refreshAllModels(providersWithKeys);
			availableModels = results;
			
			// Initialize expanded state for all providers
			Object.keys(results).forEach(provider => {
				if (expandedProviders[provider] === undefined) {
					expandedProviders[provider] = true;
				}
			});
			
			showSuccess(`Refreshed models for ${Object.keys(results).length} provider(s)`);
		} catch (error) {
			console.error('Failed to refresh models:', error);
			showError('Failed to refresh models');
		} finally {
			isRefreshing = false;
		}
	}

	async function toggleModelEnabled(provider, modelId) {
		// Find model details from available models
		const model = availableModels[provider]?.find(m => m.id === modelId);
		if (!model) return;

		try {
			const result = await modelsAPI.toggleModel(provider, modelId, model.name);
			
			// Update local state immediately for responsive UI
			const existingIndex = userModelPreferences.findIndex(
				p => p.provider === provider && p.model_id === modelId
			);

			if (existingIndex >= 0) {
				userModelPreferences[existingIndex].is_enabled = result.is_enabled;
			} else {
				userModelPreferences.push({
					provider,
					model_id: modelId,
					model_name: model.name,
					is_enabled: result.is_enabled,
					display_order: userModelPreferences.length
				});
			}
			
			userModelPreferences = [...userModelPreferences];
			dispatch('updateModels', userModelPreferences);
			
			// Also reload the enabled models store for the chat interface
			await loadEnabledModels();
			// Reload local preferences to stay in sync
			await loadUserModelPreferences();
			
			showSuccess(`${model.name} ${result.is_enabled ? 'enabled' : 'disabled'}`);
		} catch (error) {
			console.error('Failed to toggle model:', error);
			showError('Failed to toggle model');
		}
	}

	// Reactive function to check if model is enabled
	$: isModelEnabled = (provider, modelId) => {
		const pref = userModelPreferences.find(
			p => p.provider === provider && p.model_id === modelId
		);
		return pref?.is_enabled || false;
	};

	function toggleProvider(provider) {
		expandedProviders[provider] = !expandedProviders[provider];
		expandedProviders = { ...expandedProviders };
	}


	function getProviderDisplayName(provider) {
		const names = {
			openai: 'OpenAI',
			anthropic: 'Anthropic',
			openrouter: 'OpenRouter',
			google: 'Google'
		};
		return names[provider] || provider;
	}

	function getModelCount(provider, type = 'all') {
		const models = filteredModels[provider] || [];
		if (type === 'enabled') {
			return models.filter(model => isModelEnabled(provider, model.id)).length;
		}
		return models.length;
	}
</script>

<div class="models-settings">
	<div class="header">
		<h2>Model Management</h2>
		<p class="subtitle">
			Configure which AI models are available in your chat interface. Models are fetched from your configured API providers.
		</p>
	</div>

	<!-- Controls -->
	<div class="controls">
		<div class="search-container">
			<Search size={16} />
			<input
				type="text"
				placeholder="Search models..."
				bind:value={searchQuery}
				class="search-input"
			/>
		</div>

		<div class="control-buttons">
			<button
				class="control-button"
				on:click={refreshModels}
				disabled={isRefreshing || providersWithKeys.length === 0}
			>
				<RefreshCw size={16} class={isRefreshing ? 'spinning' : ''} />
				Refresh Models
			</button>

		</div>
	</div>

	<!-- Filters -->
	<div class="filters">
		<label class="filter-toggle">
			<input type="checkbox" bind:checked={showOnlyEnabled} />
			<span>Show only enabled</span>
		</label>
	</div>

	<!-- No API Keys Message -->
	{#if providersWithKeys.length === 0}
		<div class="empty-state">
			<h3>No API Keys Configured</h3>
			<p>Add API keys in the API Keys section to fetch and manage models.</p>
			<button 
				class="link-button"
				on:click={() => dispatch('navigateToApiKeys')}
			>
				Configure API Keys
			</button>
		</div>
	{:else}
		<!-- Models List -->
		<div class="models-container">
			{#each Object.entries(filteredModels) as [provider, models]}
				{#if models.length > 0}
					<div class="provider-section">
						<div class="provider-header">
							<button
								class="provider-toggle"
								on:click={() => toggleProvider(provider)}
							>
								{#if expandedProviders[provider]}
									<ChevronDown size={16} />
								{:else}
									<ChevronUp size={16} />
								{/if}
								<span class="provider-name">{getProviderDisplayName(provider)}</span>
								<span class="model-count">
									{getModelCount(provider, 'enabled')}/{getModelCount(provider)} enabled
								</span>
							</button>

						</div>

						{#if expandedProviders[provider]}
							<div class="models-grid">
								{#each models as model}
									{@const isEnabled = isModelEnabled(provider, model.id)}
									{@const shouldShow = (!showOnlyEnabled || isEnabled)}
									
									{#if shouldShow}
										<div class="model-card" class:enabled={isEnabled}>
											<div class="model-header">
												<div class="model-name" title={model.id}>
													{model.name}
												</div>
												<div class="model-actions">
													<button
														class="model-action-button"
														class:active={isEnabled}
														on:click={() => toggleModelEnabled(provider, model.id)}
														title={isEnabled ? 'Disable model' : 'Enable model'}
													>
														{#if isEnabled}
															<Check size={14} />
														{:else}
															<X size={14} />
														{/if}
													</button>
												</div>
											</div>
											
											{#if model.description}
												<div class="model-description">
													{model.description}
												</div>
											{/if}
											
											<div class="model-metadata">
												<span class="model-id">{model.id}</span>
												{#if model.context_length}
													<span class="context-length">
														{model.context_length.toLocaleString()} tokens
													</span>
												{/if}
											</div>
										</div>
									{/if}
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			{/each}

			{#if Object.keys(filteredModels).length === 0}
				<div class="empty-state">
					<h3>No Models Available</h3>
					<p>Click "Refresh Models" to fetch available models from your configured providers.</p>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.models-settings {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xl);
	}

	.header h2 {
		margin: 0 0 var(--spacing-sm) 0;
		font-size: var(--font-size-2xl);
		font-weight: 600;
		color: var(--text-primary);
	}

	.subtitle {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-relaxed);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: var(--spacing-lg);
		flex-wrap: wrap;
	}

	.search-container {
		position: relative;
		flex: 1;
		min-width: 300px;
	}

	.search-container :global(svg) {
		position: absolute;
		left: var(--spacing-md);
		top: 50%;
		transform: translateY(-50%);
		color: var(--text-muted);
	}

	.search-input {
		width: 100%;
		padding: var(--spacing-sm) var(--spacing-sm) var(--spacing-sm) 2.5rem;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		color: var(--text-primary);
		font-size: var(--font-size-sm);
	}

	.search-input:focus {
		outline: none;
		border-color: var(--border-focus);
		box-shadow: 0 0 0 3px rgba(135, 135, 135, 0.15);
	}

	.control-buttons {
		display: flex;
		gap: var(--spacing-md);
	}

	.control-button {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-lg);
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		font-weight: 500;
		cursor: pointer;
		transition: all 150ms ease-in-out;
	}

	.control-button:hover:not(:disabled) {
		background-color: var(--bg-tertiary);
		border-color: var(--border-secondary);
	}


	.control-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.filters {
		display: flex;
		align-items: center;
		gap: var(--spacing-lg);
		flex-wrap: wrap;
	}

	.filter-toggle {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
		color: var(--text-primary);
		cursor: pointer;
	}


	.models-container {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	.provider-section {
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.provider-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-md) var(--spacing-lg);
		background-color: var(--bg-secondary);
	}

	.provider-toggle {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: var(--font-size-base);
		font-weight: 600;
		cursor: pointer;
		flex: 1;
		text-align: left;
	}

	.model-count {
		font-size: var(--font-size-sm);
		color: var(--text-secondary);
		font-weight: 400;
		margin-left: var(--spacing-md);
	}

	.models-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}

	.model-card {
		background-color: var(--bg-primary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		padding: var(--spacing-md);
		transition: all 150ms ease-in-out;
	}

	.model-card:hover {
		border-color: var(--border-secondary);
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
	}

	.model-card.enabled {
		border-color: var(--accent-primary);
		background-color: var(--accent-primary-muted);
	}

	.model-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		margin-bottom: var(--spacing-sm);
	}

	.model-name {
		font-weight: 600;
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-tight);
		flex: 1;
	}

	.model-actions {
		display: flex;
		gap: var(--spacing-xs);
		margin-left: var(--spacing-sm);
	}

	.model-action-button {
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 150ms ease-in-out;
	}




	.model-description {
		font-size: var(--font-size-xs);
		color: var(--text-secondary);
		line-height: var(--line-height-relaxed);
		margin-bottom: var(--spacing-sm);
	}

	.model-metadata {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: var(--font-size-xs);
		color: var(--text-muted);
	}

	.model-id {
		font-family: var(--font-family-mono);
	}

	.context-length {
		color: var(--text-secondary);
	}

	.empty-state {
		text-align: center;
		padding: var(--spacing-3xl);
		color: var(--text-secondary);
	}

	.empty-state h3 {
		margin: 0 0 var(--spacing-md) 0;
		color: var(--text-primary);
	}

	.empty-state p {
		margin: 0 0 var(--spacing-lg) 0;
	}

	.link-button {
		display: inline-flex;
		align-items: center;
		padding: var(--spacing-sm) var(--spacing-lg);
		background-color: var(--accent-primary);
		color: white;
		text-decoration: none;
		border: none;
		border-radius: var(--radius-lg);
		font-weight: 500;
		cursor: pointer;
		font-size: var(--font-size-base);
		transition: all 150ms ease-in-out;
	}

	.link-button:hover {
		background-color: var(--accent-primary-dark, var(--accent-primary));
		opacity: 0.9;
	}

	:global(.spinning) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	@media (max-width: 768px) {
		.controls {
			flex-direction: column;
			align-items: stretch;
		}

		.control-buttons {
			justify-content: stretch;
		}

		.control-button {
			flex: 1;
		}

		.models-grid {
			grid-template-columns: 1fr;
		}

		.filters {
			flex-direction: column;
			align-items: stretch;
		}

		.provider-header {
			flex-direction: column;
			align-items: stretch;
			gap: var(--spacing-md);
		}
	}
</style>
