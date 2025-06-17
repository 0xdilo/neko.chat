<script>
	import { createEventDispatcher } from 'svelte';
	import { userName, updateUserName } from '$lib/stores/settings.js';

	const dispatch = createEventDispatcher();

	let nameValue = '';

	// Subscribe to userName store
	userName.subscribe(value => {
		nameValue = value;
	});

	function handleNameChange() {
		updateUserName(nameValue);
		dispatch('updateGeneral', { userName: nameValue });
	}
</script>

<div class="general-settings">
	<div class="settings-section">
		<h2>Profile</h2>
		<p class="settings-description">
			Customize your profile and general preferences.
		</p>

		<div class="settings-group">
			<div class="setting-row">
				<div class="setting-info">
					<label for="user-name" class="setting-label">Your Name</label>
					<p class="setting-description">
						This name will be used in welcome messages and throughout the app.
					</p>
				</div>
				<div class="setting-control">
					<input
						id="user-name"
						type="text"
						bind:value={nameValue}
						on:input={handleNameChange}
						placeholder="Enter your name"
						class="text-input"
					/>
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.general-settings {
		padding: var(--spacing-lg) 0;
		max-width: 800px;
	}

	.settings-section {
		margin-bottom: var(--spacing-2xl);
	}

	.settings-section h2 {
		font-size: var(--font-size-xl);
		font-weight: 600;
		color: var(--text-primary);
		margin-bottom: var(--spacing-sm);
	}

	.settings-description {
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
		margin-bottom: var(--spacing-xl);
		line-height: var(--line-height-normal);
	}

	.settings-group {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		padding: var(--spacing-lg);
	}

	.setting-row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-lg);
	}

	.setting-info {
		flex: 1;
		min-width: 0;
	}

	.setting-label {
		display: block;
		font-size: var(--font-size-base);
		font-weight: 500;
		color: var(--text-primary);
		margin-bottom: var(--spacing-xs);
	}

	.setting-description {
		font-size: var(--font-size-sm);
		color: var(--text-secondary);
		line-height: var(--line-height-normal);
	}

	.setting-control {
		flex-shrink: 0;
		min-width: 200px;
	}

	.text-input {
		width: 100%;
		padding: var(--spacing-sm) var(--spacing-md);
		background-color: var(--bg-primary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-md);
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		transition: all var(--transition-fast);
	}

	.text-input:focus {
		outline: none;
		border-color: var(--border-focus);
		box-shadow: 0 0 0 3px rgba(135, 135, 135, 0.15);
	}

	.text-input::placeholder {
		color: var(--text-muted);
	}

	@media (max-width: 768px) {
		.setting-row {
			flex-direction: column;
			gap: var(--spacing-md);
		}

		.setting-control {
			min-width: unset;
			width: 100%;
		}
	}
</style>