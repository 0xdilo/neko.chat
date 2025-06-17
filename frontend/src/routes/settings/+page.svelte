<script>
	import AppLayout from "$lib/components/AppLayout.svelte";
	import SettingsSidebar from "$lib/components/SettingsSidebar.svelte";
	import General from "$lib/components/settings/General.svelte";
	import ApiKeys from "$lib/components/settings/ApiKeys.svelte";
	import Models from "$lib/components/settings/Models.svelte";
	import Appearance from "$lib/components/settings/Appearance.svelte";
	import Shortcuts from "$lib/components/settings/Shortcuts.svelte";
	import SystemPrompts from "$lib/components/settings/SystemPrompts.svelte";
	import { Save, Check } from "lucide-svelte";
	import { fly } from "svelte/transition";
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { apiKeys as apiKeysStore } from '$lib/stores/settings.js';

	let activeSection = "general";
	let saveState = 'idle'; // 'idle' | 'saving' | 'saved'
	let saveTimeout;

	const sectionComponents = {
		general: General,
		"api-keys": ApiKeys,
		models: Models,
		prompts: SystemPrompts,
		shortcuts: Shortcuts,
		appearance: Appearance,
	};

	// Valid sections for URL params
	const validSections = Object.keys(sectionComponents);

	// Initialize section from URL params
	onMount(() => {
		if (browser) {
			const urlSection = $page.url.searchParams.get('section');
			if (urlSection && validSections.includes(urlSection)) {
				activeSection = urlSection;
			}
		}
	});

	// Reactive statement to update URL when activeSection changes
	$: if (browser && activeSection) {
		updateURL(activeSection);
	}

	function updateURL(section) {
		const url = new URL(window.location);
		url.searchParams.set('section', section);
		goto(url.toString(), { replaceState: true, noScroll: true });
	}

	function handleSectionSelect(event) {
		activeSection = event.detail;
	}

	// Use real store data instead of mock data
	$: apiKeys = $apiKeysStore;

	let systemPrompts = [
		{
			id: 1,
			name: "Default Assistant",
			prompt: "You are a helpful AI assistant.",
			active: true,
		},
		{
			id: 2,
			name: "Code Helper",
			prompt: "You are an expert software developer. Help with coding tasks.",
			active: false,
		},
	];

	let keyboardShortcuts = [
		{ id: 1, action: "New Chat", keys: ["Ctrl", "O"] },
		{ id: 2, action: "Focus Search", keys: ["Ctrl", "K"] },
		{ id: 3, action: "Toggle Sidebar", keys: ["Ctrl", "B"] },
		{ id: 4, action: "Open Model Selector", keys: ["Ctrl", "M"] },
		{ id: 5, action: "Settings", keys: ["Ctrl", ","] },
	];

	let appearanceSettings = {
		fontSize: "medium",
		messageAnimation: true,
		compactMode: false,
	};

	const handlers = {
		updateGeneral: (e) => {
			// General settings are automatically saved in the component
			triggerSave();
		},
		updateApiKeys: (e) => {
			apiKeys = e.detail;
			triggerSave();
		},
		updateModels: (e) => {
			// Models are saved in the component
			triggerSave();
		},
		updateSystemPrompts: (e) => {
			systemPrompts = e.detail;
			triggerSave();
		},
		updateAppearance: (e) => {
			appearanceSettings = e.detail;
			triggerSave();
		},
		navigateToApiKeys: (e) => {
			activeSection = 'api-keys';
		},
	};

	function triggerSave() {
		saveState = 'saving';
		clearTimeout(saveTimeout);
		saveTimeout = setTimeout(() => {
			saveState = 'saved';
			setTimeout(() => {
				if (saveState === 'saved') saveState = 'idle';
			}, 2000);
		}, 800);
	}

	function saveAllSettings() {
		triggerSave();
		console.log("saving all settings...", {
			apiKeys,
			systemPrompts,
			appearanceSettings,
		});
	}
</script>

<svelte:head>
	<title>Settings - neko.chat</title>
</svelte:head>

<AppLayout>
	<div slot="sidebar">
		<SettingsSidebar
			bind:activeSection
			on:select={handleSectionSelect}
		/>
	</div>

	<div class="settings-page">
		<div class="settings-content">
			<svelte:component
				this={sectionComponents[activeSection]}
				{apiKeys}
				{systemPrompts}
				{keyboardShortcuts}
				{appearanceSettings}
				on:updateGeneral={handlers.updateGeneral}
				on:updateApiKeys={handlers.updateApiKeys}
				on:updateModels={handlers.updateModels}
				on:updateSystemPrompts={handlers.updateSystemPrompts}
				on:updateAppearance={handlers.updateAppearance}
				on:navigateToApiKeys={handlers.navigateToApiKeys}
			/>
		</div>

		{#if saveState !== 'idle'}
			<div class="save-indicator" transition:fly={{ y: 20, duration: 200 }}>
				{#if saveState === 'saving'}
					<div class="save-spinner"></div>
					<span>saving changes...</span>
				{:else}
					<Check size={16} />
					<span>changes saved</span>
				{/if}
			</div>
		{/if}
	</div>
</AppLayout>

<style>
	.settings-page {
		height: 100vh;
		display: flex;
		flex-direction: column;
		background-color: var(--bg-primary);
		position: relative;
		overflow: hidden;
	}

	.settings-content {
		flex: 1;
		overflow-y: auto;
		padding: var(--spacing-2xl) var(--spacing-3xl);
		max-width: 1200px;
		margin: 0 auto;
		width: 100%;
	}

	.save-indicator {
		position: fixed;
		bottom: var(--spacing-xl);
		right: var(--spacing-xl);
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-lg);
		background-color: var(--bg-elevated);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-full);
		font-size: var(--font-size-sm);
		font-weight: 500;
		color: var(--text-primary);
		box-shadow: var(--shadow-lg);
		backdrop-filter: blur(12px);
		z-index: 100;
	}

	.save-spinner {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border-primary);
		border-top: 2px solid var(--accent-primary);
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	@media (max-width: 768px) {
		.settings-content {
			padding: var(--spacing-xl) var(--spacing-lg);
		}
		
		.save-indicator {
			bottom: var(--spacing-lg);
			right: var(--spacing-lg);
		}
	}
</style>
