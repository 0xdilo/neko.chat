<script>
	import { createEventDispatcher, onMount } from 'svelte';
	import { clickOutside } from '$lib/actions/clickOutside.js';

	export let selectedModels = [];
	export let availableProviders = [];
	export let showDropdown = false;

	const dispatch = createEventDispatcher();

	function toggleDropdown() {
		showDropdown = !showDropdown;
		dispatch('toggle', { show: showDropdown });
	}

	function handleSelectionChange(provider, modelId) {
		// Find existing selection for this provider
		const existingIndex = selectedModels.findIndex(m => m.startsWith(provider + ':'));
		const newSelection = `${provider}:${modelId}`;
		
		let newSelectedModels;
		if (existingIndex >= 0) {
			// Replace existing selection for this provider
			newSelectedModels = [...selectedModels];
			newSelectedModels[existingIndex] = newSelection;
		} else {
			// Add new selection
			newSelectedModels = [...selectedModels, newSelection];
		}
		
		dispatch('selectionChange', { selectedModels: newSelectedModels });
	}

	function getDisplayName() {
		if (selectedModels.length === 0) return 'Select Model';
		if (selectedModels.length === 1) {
			const [provider, modelId] = selectedModels[0].split(':');
			const providerData = availableProviders.find(p => p.name === provider);
			const model = providerData?.models?.find(m => m.id === modelId);
			return model?.name || modelId || 'Unknown Model';
		}
		return `${selectedModels.length} models selected`;
	}

	function isModelSelected(provider, modelId) {
		return selectedModels.some(m => m === `${provider}:${modelId}`);
	}

	function handleClickOutside() {
		showDropdown = false;
		dispatch('toggle', { show: false });
	}
</script>

<div class="model-selector">
	<button
		class="model-selector-button"
		on:click={toggleDropdown}
		class:active={showDropdown}
	>
		<span class="model-name">{getDisplayName()}</span>
		<svg
			class="chevron"
			class:rotated={showDropdown}
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
		>
			<polyline points="6,9 12,15 18,9"></polyline>
		</svg>
	</button>

	{#if showDropdown}
		<div
			class="model-selector-dropdown"
			use:clickOutside={handleClickOutside}
			on:click|stopPropagation
			on:keydown
			role="menu"
		>
			{#each availableProviders as provider}
				{#if provider.models && provider.models.length > 0}
					<div class="provider-section">
						<div class="provider-header">
							<span class="provider-name">{provider.display_name || provider.name}</span>
						</div>
						{#each provider.models as model}
							<button
								class="model-option"
								class:selected={isModelSelected(provider.name, model.id)}
								on:click={() => handleSelectionChange(provider.name, model.id)}
								role="menuitem"
							>
								<div class="model-info">
									<span class="model-id">{model.name || model.id}</span>
									{#if model.description}
										<span class="model-description">{model.description}</span>
									{/if}
								</div>
								{#if isModelSelected(provider.name, model.id)}
									<div class="selected-indicator">
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<polyline points="20,6 9,17 4,12"></polyline>
										</svg>
									</div>
								{/if}
							</button>
						{/each}
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style>
	.model-selector {
		position: relative;
		display: inline-block;
	}

	.model-selector-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		color: var(--text-primary);
		cursor: pointer;
		transition: all 0.2s ease;
		min-width: 160px;
		justify-content: space-between;
	}

	.model-selector-button:hover {
		background: var(--bg-tertiary);
	}

	.model-selector-button.active {
		border-color: var(--accent-color);
	}

	.model-name {
		font-size: 14px;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.chevron {
		transition: transform 0.2s ease;
		flex-shrink: 0;
	}

	.chevron.rotated {
		transform: rotate(180deg);
	}

	.model-selector-dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		z-index: 1000;
		max-height: 400px;
		overflow-y: auto;
		margin-top: 4px;
	}

	.provider-section {
		border-bottom: 1px solid var(--border-color);
	}

	.provider-section:last-child {
		border-bottom: none;
	}

	.provider-header {
		padding: 0.75rem 1rem 0.5rem;
		background: var(--bg-secondary);
		position: sticky;
		top: 0;
		z-index: 1;
	}

	.provider-name {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		color: var(--text-secondary);
		letter-spacing: 0.05em;
	}

	.model-option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 0.75rem 1rem;
		background: none;
		border: none;
		text-align: left;
		cursor: pointer;
		transition: background 0.2s ease;
		color: var(--text-primary);
	}

	.model-option:hover {
		background: var(--bg-secondary);
	}

	.model-option.selected {
		background: var(--accent-color-light);
		color: var(--accent-color);
	}

	.model-info {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
	}

	.model-id {
		font-size: 14px;
		font-weight: 500;
	}

	.model-description {
		font-size: 12px;
		color: var(--text-secondary);
		line-height: 1.3;
	}

	.selected-indicator {
		color: var(--accent-color);
		flex-shrink: 0;
	}
</style>