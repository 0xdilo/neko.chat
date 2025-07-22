<script>
	import { createEventDispatcher } from 'svelte';
	import { X } from 'lucide-svelte';
	import ModelSelector from './ModelSelector.svelte';

	export let show = false;
	export let messageId = null;
	export let availableProviders = [];

	const dispatch = createEventDispatcher();

	let selectedModel = '';
	let branchType = 'current'; // 'current' or 'alternative'

	function handleClose() {
		show = false;
		dispatch('close');
		resetForm();
	}

	function handleCreateBranch() {
		if (branchType === 'current') {
			dispatch('createBranch', { 
				type: 'current',
				messageId 
			});
		} else {
			dispatch('createBranch', { 
				type: 'alternative',
				messageId,
				model: selectedModel
			});
		}
		handleClose();
	}

	function handleModelSelection(event) {
		const { selectedModels } = event.detail;
		selectedModel = selectedModels[0] || '';
	}

	function resetForm() {
		selectedModel = '';
		branchType = 'current';
	}

	$: canCreate = branchType === 'current' || (branchType === 'alternative' && selectedModel);
</script>

{#if show}
	<div class="modal-overlay" on:click={handleClose} on:keydown>
		<div class="modal-content" on:click|stopPropagation on:keydown>
			<div class="modal-header">
				<h3>Create Conversation Branch</h3>
				<button class="close-button" on:click={handleClose}>
					<X size={20} />
				</button>
			</div>

			<div class="modal-body">
				<p class="description">
					Choose how to branch this conversation from the selected message.
				</p>

				<div class="branch-options">
					<label class="branch-option">
						<input 
							type="radio" 
							bind:group={branchType} 
							value="current" 
						/>
						<div class="option-content">
							<strong>Continue with current model</strong>
							<span>Create a branch using the same model as the current conversation</span>
						</div>
					</label>

					<label class="branch-option">
						<input 
							type="radio" 
							bind:group={branchType} 
							value="alternative" 
						/>
						<div class="option-content">
							<strong>Try with different model</strong>
							<span>Create a branch and generate alternative response with a different model</span>
						</div>
					</label>
				</div>

				{#if branchType === 'alternative'}
					<div class="model-selection">
						<label class="form-label">Select Model:</label>
						<ModelSelector 
							{availableProviders}
							selectedModels={selectedModel ? [selectedModel] : []}
							on:selectionChange={handleModelSelection}
						/>
					</div>
				{/if}
			</div>

			<div class="modal-footer">
				<button class="cancel-button" on:click={handleClose}>
					Cancel
				</button>
				<button 
					class="create-button" 
					on:click={handleCreateBranch}
					disabled={!canCreate}
				>
					Create Branch
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.modal-content {
		background: var(--bg-primary);
		border-radius: 12px;
		border: 1px solid var(--border-color);
		width: 100%;
		max-width: 500px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-color);
	}

	.modal-header h3 {
		margin: 0;
		font-size: 18px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.close-button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		background: none;
		cursor: pointer;
		border-radius: 6px;
		color: var(--text-secondary);
		transition: all 0.2s ease;
	}

	.close-button:hover {
		background: var(--bg-secondary);
		color: var(--text-primary);
	}

	.modal-body {
		padding: 1.5rem;
	}

	.description {
		color: var(--text-secondary);
		margin: 0 0 1.5rem 0;
		line-height: 1.5;
	}

	.branch-options {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.branch-option {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 1rem;
		border: 2px solid var(--border-color);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.branch-option:hover {
		border-color: var(--accent-color);
		background: var(--bg-secondary);
	}

	.branch-option:has(input:checked) {
		border-color: var(--accent-color);
		background: var(--accent-color-light);
	}

	.branch-option input[type="radio"] {
		margin-top: 2px;
	}

	.option-content {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.option-content strong {
		color: var(--text-primary);
		font-size: 14px;
	}

	.option-content span {
		color: var(--text-secondary);
		font-size: 13px;
		line-height: 1.4;
	}

	.model-selection {
		margin-top: 1rem;
	}

	.form-label {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 14px;
		font-weight: 500;
		color: var(--text-primary);
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid var(--border-color);
	}

	.cancel-button,
	.create-button {
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.cancel-button {
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
	}

	.cancel-button:hover {
		background: var(--bg-tertiary);
	}

	.create-button {
		background: var(--accent-color);
		border: 1px solid var(--accent-color);
		color: white;
	}

	.create-button:hover:not(:disabled) {
		background: var(--accent-color-dark);
		border-color: var(--accent-color-dark);
	}

	.create-button:disabled {
		background: var(--bg-tertiary);
		border-color: var(--border-color);
		color: var(--text-disabled);
		cursor: not-allowed;
	}
</style>