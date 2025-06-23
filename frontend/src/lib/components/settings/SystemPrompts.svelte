<script>
	import { createEventDispatcher, onMount } from "svelte";
	import { slide } from "svelte/transition";
	import { Plus, Trash2, Edit, Save, X, CheckCircle } from "lucide-svelte";
	import { settingsAPI } from "$lib/api/settings.js";
	import { showError, showSuccess } from "$lib/stores/app.js";

	export let systemPrompts = [];
	const dispatch = createEventDispatcher();

	let selectedIds = new Set();
	let lastClickedId = null;
	let mode = "view"; // 'view', 'edit', 'new'
	let promptBuffer = {}; // temporary state for edits/new
	let isLoading = false;

	onMount(async () => {
		await loadSystemPrompts();
	});

	async function loadSystemPrompts() {
		isLoading = true;
		try {
			systemPrompts = await settingsAPI.getSystemPrompts();
		} catch (error) {
			console.error("failed to load system prompts:", error);
			showError("failed to load system prompts");
		} finally {
			isLoading = false;
		}
	}

	// --- derived state ---
	$: singleSelectedId =
		selectedIds.size === 1 ? [...selectedIds][0] : null;
	$: singleSelectedPrompt = singleSelectedId
		? systemPrompts.find((p) => p.id === singleSelectedId)
		: null;

	// --- event handlers ---
	function handlePromptClick(clickedId, event) {
		if (mode === "edit" || isLoading) return;

		const { ctrlKey, metaKey, shiftKey } = event;

		if (shiftKey && lastClickedId) {
			const allIds = systemPrompts.map((p) => p.id);
			const lastIndex = allIds.indexOf(lastClickedId);
			const currentIndex = allIds.indexOf(clickedId);
			const [start, end] = [lastIndex, currentIndex].sort((a, b) => a - b);
			const rangeIds = allIds.slice(start, end + 1);

			if (ctrlKey || metaKey) {
				rangeIds.forEach((id) => selectedIds.add(id));
			} else {
				selectedIds = new Set(rangeIds);
			}
		} else if (ctrlKey || metaKey) {
			selectedIds.has(clickedId)
				? selectedIds.delete(clickedId)
				: selectedIds.add(clickedId);
		} else {
			selectedIds = new Set([clickedId]);
		}

		lastClickedId = clickedId;
		selectedIds = selectedIds; // force reactivity
		mode = "view";
	}

	function handleAddNew() {
		selectedIds.clear();
		lastClickedId = null;
		promptBuffer = {
			name: "",
			prompt: "",
			description: "",
			category: "general",
		};
		mode = "new";
	}

	function handleEdit() {
		if (!singleSelectedPrompt) return;
		promptBuffer = structuredClone(singleSelectedPrompt);
		mode = "edit";
	}

	async function handleSave() {
		if (!promptBuffer.name?.trim() || !promptBuffer.prompt?.trim()) {
			showError("name and prompt are required.");
			return;
		}

		isLoading = true;
		try {
			const payload = {
				name: promptBuffer.name,
				prompt: promptBuffer.prompt,
				description: promptBuffer.description || "",
				category: promptBuffer.category || "general",
			};

			if (mode === "edit") {
				await settingsAPI.updateSystemPrompt(promptBuffer.id, payload);
				showSuccess("system prompt updated.");
			} else {
				await settingsAPI.addSystemPrompt({
					...payload,
					isDefault: systemPrompts.length === 0,
				});
				showSuccess("system prompt created.");
			}
			await loadSystemPrompts();
			mode = "view";
			promptBuffer = {};
		} catch (error) {
			console.error("failed to save system prompt:", error);
			showError("failed to save system prompt");
		} finally {
			isLoading = false;
		}
	}

	function handleCancel() {
		if (mode === "new") selectedIds.clear();
		mode = "view";
		promptBuffer = {};
	}

	async function handleSingleDelete(promptId) {
		const promptToDelete = systemPrompts.find((p) => p.id === promptId);
		if (promptToDelete?.is_default) {
			showError("cannot delete the default prompt.");
			return;
		}

		isLoading = true;
		try {
			await settingsAPI.deleteSystemPrompt(promptId);
			showSuccess("prompt deleted.");
			selectedIds.delete(promptId);
			selectedIds = selectedIds; // reactivity
			await loadSystemPrompts();
		} catch (error) {
			console.error(`failed to delete prompt ${promptId}:`, error);
			showError("failed to delete prompt.");
		} finally {
			isLoading = false;
		}
	}

	async function handleBulkRemove() {
		const idsToDelete = [...selectedIds].filter(
			(id) => !systemPrompts.find((p) => p.id === id)?.is_default,
		);

		if (idsToDelete.length === 0) {
			showError("cannot delete default prompts or selection is empty.");
			return;
		}

		isLoading = true;
		try {
			await Promise.all(
				idsToDelete.map((id) => settingsAPI.deleteSystemPrompt(id)),
			);
			showSuccess(`deleted ${idsToDelete.length} prompt(s).`);
			selectedIds.clear();
			await loadSystemPrompts();
		} catch (error) {
			console.error("failed to delete system prompts:", error);
			showError("failed to delete system prompts");
		} finally {
			isLoading = false;
		}
	}

	async function handleToggleDefault(promptId) {
		isLoading = true;
		try {
			await settingsAPI.toggleActivePrompt(promptId);
			showSuccess("default prompt status updated.");
			await loadSystemPrompts();
		} catch (error) {
			console.error("failed to toggle default prompt:", error);
			showError("failed to toggle default prompt");
		} finally {
			isLoading = false;
		}
	}
</script>

<div class="section-header">
	<h2 class="section-title">system prompts</h2>
	<p class="section-description">
		tailor the ai's personality and expertise for different tasks.
	</p>
</div>

<div class="prompts-layout">
	<div class="prompts-list-panel">
		<div class="prompts-list">
			{#if isLoading && systemPrompts.length === 0}
				<div class="loading-state">
					<div class="loading-spinner" >
					</div>
					<p>loading system prompts...</p>
				</div>
			{:else}
				<nav>
					{#each systemPrompts as prompt (prompt.id)}
						<div
							role="button"
							tabindex="0"
							class="prompt-list-item"
							class:selected={selectedIds.has(prompt.id)}
							on:click={(e) => handlePromptClick(prompt.id, e)}
							on:keydown={(e) =>
								e.key === "enter" && handlePromptClick(prompt.id, e)}
							class:disabled={mode === 'edit' || isLoading}
						>
							<span class="prompt-name">{prompt.name}</span>
							<div class="item-icons">
								{#if prompt.is_default}
									<CheckCircle size={14} class="active-icon" title="Default prompt" />
								{/if}
								<button
									class="delete-icon"
									title="Delete prompt"
									on:click|stopPropagation={() => handleSingleDelete(prompt.id)}
									disabled={prompt.is_default || isLoading}
								>
									<Trash2 size={14} />
								</button>
							</div>
						</div>
					{:else}
						<div class="empty-state">
							<p>no system prompts yet.</p>
							<small>create your first prompt to get started.</small>
						</div>
					{/each}
				</nav>
			{/if}
		</div>
		<div class="panel-footer">
			<button
				class="add-new-btn"
				on:click={handleAddNew}
				disabled={mode === 'edit' || isLoading}
			>
				<Plus size={16} />
				<span>new prompt</span>
			</button>
		</div>
	</div>

	<div class="prompt-detail-panel">
		{#if selectedIds.size > 1}
			<div class="bulk-actions-panel" transition:slide|local>
				<h3 class="bulk-title">{selectedIds.size} prompts selected</h3>
				<p class="bulk-description">
					you can perform bulk actions on the selected items. default prompts
					cannot be deleted.
				</p>
				<button class="action-btn danger" on:click={handleBulkRemove}>
					<Trash2 size={16} />
					delete selected
				</button>
			</div>
		{:else if singleSelectedPrompt && mode === 'view'}
			<div class="prompt-viewer" transition:slide|local>
				<div class="viewer-header">
					<h3 class="viewer-title">{singleSelectedPrompt.name}</h3>
					<div class="viewer-actions">
						<button
							class="action-btn {singleSelectedPrompt.is_default ? 'deactivate' : 'activate'}"
							on:click={() => handleToggleDefault(singleSelectedPrompt.id)}
							>{singleSelectedPrompt.is_default ? 'Deactivate' : 'Activate'}</button
						>
						<button class="action-btn" on:click={handleEdit}
							><Edit size={16} /></button
						>
					</div>
				</div>
				<div class="prompt-content">
					<p>{singleSelectedPrompt.prompt}</p>
				</div>
			</div>
		{:else if (singleSelectedPrompt && mode === 'edit') || mode === 'new'}
			<div class="prompt-editor" transition:slide|local>
				<div class="form-group">
					<label for="prompt-name">name</label>
					<input
						id="prompt-name"
						bind:value={promptBuffer.name}
						placeholder="e.g., code reviewer"
					/>
				</div>
				<div class="form-group form-group-stretch">
					<label for="prompt-content">prompt</label>
					<textarea
						id="prompt-content"
						bind:value={promptBuffer.prompt}
						placeholder="you are an expert..."
					>
					</textarea>
				</div>
				<div class="editor-actions">
					<button class="cancel-btn" on:click={handleCancel}>
						<X size={16} /> cancel
					</button>
					<button class="save-btn" on:click={handleSave}>
						<Save size={16} /> save
					</button>
				</div>
			</div>
		{:else}
			<div class="placeholder-panel">
				<p>select a prompt or create a new one.</p>
				<p class="subtle-text">
					(use ctrl/cmd or shift to select multiple)
				</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.section-header {
		margin-bottom: var(--spacing-xl);
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
	.prompts-layout {
		display: grid;
		grid-template-columns: minmax(250px, 1fr) 2.5fr;
		gap: var(--spacing-lg);
		height: calc(100vh - 350px);
	}
	.prompts-list-panel {
		display: flex;
		flex-direction: column;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}
	.prompts-list {
		flex: 1;
		overflow-y: auto;
		padding: var(--spacing-sm);
	}
	.prompt-list-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border-radius: var(--radius-lg);
		text-align: left;
		transition: background-color 150ms;
		color: var(--text-secondary);
		cursor: pointer;
		border: 1px solid transparent;
	}
	.prompt-list-item:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}
	.prompt-list-item.selected {
		background-color: var(--interactive-selected-muted);
		color: var(--text-primary);
		font-weight: 600;
		border-color: var(--accent-primary);
	}
	.prompt-list-item.disabled {
		opacity: 0.6;
		cursor: not-allowed;
		background-color: transparent;
	}
	.prompt-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex-grow: 1;
		border-radius: var(--radius-lg)

	}
	.item-icons {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex-shrink: 0;
	}
	.delete-icon {
		visibility: hidden;
		opacity: 0;
		transition:
			opacity 150ms,
			visibility 150ms;
		background: none;
		border: none;
		padding: 2px;
		cursor: pointer;
		color: var(--text-secondary);
		border-radius: var(--radius-sm);
		display: flex;
		align-items: center;
	}
	.prompt-list-item:hover .delete-icon {
		visibility: visible;
		opacity: 1;
	}
	.delete-icon:hover:not(:disabled) {
		color: var(--status-error);
		background-color: var(--status-error-muted);
	}
	.delete-icon:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}
	.panel-footer {
		padding: var(--spacing-md);
		border-top: 1px solid var(--border-primary);
	}
	.add-new-btn {
		width: 100%;
		display: flex;
		justify-content: center;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm);
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		font-weight: 500;
	}
	.prompt-detail-panel {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		padding: var(--spacing-xl);
		overflow-y: auto;
	}
	.placeholder-panel,
	.bulk-actions-panel {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		text-align: center;
		gap: var(--spacing-md);
	}
	.placeholder-panel .subtle-text {
		color: var(--text-muted);
		font-size: var(--font-size-sm);
	}
	.bulk-title {
		font-size: var(--font-size-xl);
		font-weight: 600;
	}
	.bulk-description {
		color: var(--text-secondary);
		max-width: 40ch;
	}
	.viewer-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--spacing-lg);
	}
	.viewer-title {
		font-size: var(--font-size-xl);
		font-weight: 600;
		color: var(--text-primary);
		text-transform: capitalize;
	}
	.viewer-actions {
		display: flex;
		gap: var(--spacing-sm);
	}
	.action-btn {
		padding: var(--spacing-sm);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
	}
	.action-btn.activate {
		background-color: var(--accent-primary-muted);
		color: var(--accent-primary);
		border-color: var(--accent-primary);
	}
	.action-btn.deactivate {
		background-color: var(--status-warning-muted);
		color: var(--status-warning);
		border-color: var(--status-warning);
	}
	.action-btn.danger:not(:disabled):hover {
		background-color: var(--status-error-muted);
		color: var(--status-error);
		border-color: var(--status-error);
	}
	.prompt-content {
		background-color: var(--bg-primary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		padding: var(--spacing-lg);
		white-space: pre-wrap;
		font-family: var(--font-family-mono);
		color: var(--text-secondary);
		line-height: 1.6;
	}
	.prompt-editor {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		height: 100%;
	}
	.form-group label {
		display: block;
		font-weight: 500;
		margin-bottom: var(--spacing-sm);
		text-transform: capitalize;
		border-radius: var(--radius-lg)

	}
	.form-group-stretch {
		flex-grow: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.form-group-stretch textarea {
		height: 100%; 
		resize: none;
	}
	.editor-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-md);
	}
	.cancel-btn {
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg)
	}
	.save-btn {
		background-color: var(--accent-primary);
		color: var(--bg-primary);
		border-radius: var(--radius-lg)

	}
	.loading-state,
	.empty-state {
		text-align: center;
		padding: var(--spacing-2xl);
		color: var(--text-secondary);
	}
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-md);
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
	.empty-state small {
		color: var(--text-muted);
		font-size: var(--font-size-sm);
	}
</style>
