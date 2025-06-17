
<script>
	import { onMount, onDestroy } from "svelte";
	import {
		createKeybindingSystem,
		commandSequence,
		keybindingMode,
	} from "$lib/keybindings.js";
	import { theme } from "$lib/theme.js";
	import { sidebarCollapsed } from "$lib/stores/ui.js"; // a store is the correct tool here.
	import { createChat } from "$lib/stores/chats.js";

	let keybindingSystem;
	let showStatus = false;
	let statusTimeout;

	onMount(() => {
		keybindingSystem = createKeybindingSystem();

		if (typeof window !== "undefined") {
			window.addEventListener("toggle-sidebar", handleToggleSidebar);
			window.addEventListener("toggle-theme", handleToggleTheme);
			window.addEventListener("new-chat", handleNewChat);
			window.addEventListener("escape-action", handleEscape);
			window.addEventListener("command-palette", handleCommandPalette);
			window.addEventListener("model-modal", handleModelModal);
			window.addEventListener("focus-input", handleFocusInput); // listener for input focus
		}

		const unsubscribe = commandSequence.subscribe((sequence) => {
			if (sequence) {
				showStatus = true;
				clearTimeout(statusTimeout);
				statusTimeout = setTimeout(() => {
					showStatus = false;
				}, 2000);
			} else {
				showStatus = false;
			}
		});

		return () => {
			unsubscribe();
		};
	});

	onDestroy(() => {
		if (keybindingSystem) {
			keybindingSystem.destroy();
		}

		if (typeof window !== "undefined") {
			window.removeEventListener("toggle-sidebar", handleToggleSidebar);
			window.removeEventListener("toggle-theme", handleToggleTheme);
			window.removeEventListener("new-chat", handleNewChat);
			window.removeEventListener("escape-action", handleEscape);
			window.removeEventListener(
				"command-palette",
				handleCommandPalette,
			);
			window.removeEventListener("model-modal", handleModelModal);
			window.removeEventListener("focus-input", handleFocusInput); // cleanup
		}

		if (typeof clearTimeout !== "undefined") {
			clearTimeout(statusTimeout);
		}
	});

	function handleToggleSidebar() {
		sidebarCollapsed.update((v) => !v);
	}

	function handleToggleTheme() {
		theme.toggle();
	}

	async function handleNewChat() {
		try {
			await createChat();
		} catch (error) {
			console.error("Failed to create new chat:", error);
		}
	}

	function handleEscape() {
		if (typeof document !== "undefined") {
			const activeElement = document.activeElement;
			if (activeElement && activeElement.blur) {
				activeElement.blur();
			}
		}
	}

	function handleCommandPalette() {
		console.log("opening command palette...");
	}

	function handleModelModal() {
		window.dispatchEvent(new CustomEvent("toggle-model-selector"));
	}

	function handleFocusInput() {
		if (typeof document !== "undefined") {
			const input = document.getElementById("chat-input");
			input?.focus();
		}
	}
</script>

{#if showStatus && $commandSequence}
	<div class="keybinding-status">
		<div class="status-content">
			<span class="status-label">command:</span>
			<code class="status-sequence">{$commandSequence}</code>
		</div>
	</div>
{/if}

{#if $keybindingMode === 'help'}
	<div class="keybinding-help-overlay">
		<div class="help-content">
			<h3>keyboard shortcuts</h3>
			<div class="help-sections">
				<div class="help-section">
					<h4>navigation</h4>
					<div class="shortcuts">
						<div class="shortcut">
							<kbd>j</kbd>
							<span>scroll down</span>
						</div>
						<div class="shortcut">
							<kbd>k</kbd>
							<span>scroll up</span>
						</div>
						<div class="shortcut">
							<kbd>gg</kbd>
							<span>go to top</span>
						</div>
						<div class="shortcut">
							<kbd>g</kbd>
							<span>go to bottom</span>
						</div>
						<div class="shortcut">
							<kbd>h</kbd>
							<span>toggle sidebar</span>
						</div>
						<div class="shortcut">
							<kbd>x</kbd>
							<span>toggle sidebar</span>
						</div>
					</div>
				</div>

				<div class="help-section">
					<h4>chat actions</h4>
					<div class="shortcuts">
						<div class="shortcut">
							<kbd>o</kbd>
							<span>new chat</span>
						</div>
						<div class="shortcut">
							<kbd>i</kbd>
							<span>focus input</span>
						</div>
						<div class="shortcut">
							<kbd>/</kbd>
							<span>search chats</span>
						</div>
					</div>
				</div>

				<div class="help-section">
					<h4>global shortcuts</h4>
					<div class="shortcuts">
						<div class="shortcut">
							<kbd>ctrl+n</kbd>
							<span>new chat</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+o</kbd>
							<span>new chat</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+k</kbd>
							<span>command palette</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+m</kbd>
							<span>model modal</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+b</kbd>
							<span>toggle sidebar</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+,</kbd>
							<span>settings</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+t</kbd>
							<span>toggle theme</span>
						</div>
						<div class="shortcut">
							<kbd>ctrl+i</kbd>
							<span>focus input</span>
						</div>
					</div>
				</div>
			</div>
			<div class="help-footer">
				<p>press <kbd>escape</kbd> to close this help</p>
			</div>
		</div>
	</div>
{/if}

<slot />

<style>
	.keybinding-status {
		position: fixed;
		top: var(--spacing-lg);
		right: var(--spacing-lg);
		z-index: 1000;
		pointer-events: none;
	}

	.status-content {
		background-color: var(--bg-elevated);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm) var(--spacing-md);
		box-shadow: var(--shadow-md);
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		animation: fade-in 0.2s ease-out;
	}

	.status-label {
		font-size: var(--font-size-sm);
		color: var(--text-secondary);
		font-weight: 500;
	}

	.status-sequence {
		font-family: var(--font-family-mono);
		background-color: var(--bg-tertiary);
		padding: var(--spacing-xs) var(--spacing-sm);
		border-radius: var(--radius-sm);
		color: var(--text-primary);
		font-weight: 600;
	}

	.keybinding-help-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: rgba(0, 0, 0, 0.8);
		backdrop-filter: blur(4px);
		z-index: 2000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-lg);
		animation: fade-in 0.3s ease-out;
	}

	.help-content {
		background-color: var(--bg-primary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		padding: var(--spacing-xl);
		max-width: 800px;
		max-height: 80vh;
		overflow-y: auto;
		box-shadow: var(--shadow-lg);
	}

	.help-content h3 {
		font-size: var(--font-size-xl);
		color: var(--text-primary);
		margin-bottom: var(--spacing-lg);
		text-align: center;
		font-family: var(--font-family-mono);
	}

	.help-sections {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: var(--spacing-xl);
	}

	.help-section h4 {
		font-size: var(--font-size-lg);
		color: var(--text-primary);
		margin-bottom: var(--spacing-md);
		font-weight: 600;
	}

	.shortcuts {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.shortcut {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-sm);
		background-color: var(--bg-secondary);
		border-radius: var(--radius-sm);
	}

	.shortcut kbd {
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--text-primary);
		min-width: 2rem;
		text-align: center;
	}

	.shortcut span {
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}

	.help-footer {
		margin-top: var(--spacing-xl);
		text-align: center;
		color: var(--text-muted);
		font-size: var(--font-size-sm);
	}

	.help-footer kbd {
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		font-family: var(--font-family-mono);
		color: var(--text-primary);
	}

	@keyframes fade-in {
		from {
			opacity: 0;
			transform: translateY(-10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@media (max-width: 768px) {
		.help-sections {
			grid-template-columns: 1fr;
		}

		.keybinding-status {
			top: var(--spacing-md);
			right: var(--spacing-md);
		}
	}
</style>
