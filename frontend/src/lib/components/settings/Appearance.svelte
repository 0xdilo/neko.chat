<script>
	import { theme as themeStore, themes } from "$lib/theme.js";
	import { CheckCircle2, Sun, Moon } from "lucide-svelte";

	// this data should probably live in a global config/theme file
	const availableThemes = [
		{
			id: "neko-dark",
			name: "neko dark",
			colors: ["#A3A3A3", "#27272A", "#4ADE80", "#F4F4F5"],
		},
		{
			id: "tokyonight",
			name: "tokyo night",
			colors: ["#a9b1d6", "#1a1b26", "#7aa2f7", "#c0caf5"],
		},
		{
			id: "catppuccin-latte",
			name: "catppuccin latte",
			colors: ["#4c4f69", "#eff1f5", "#1e66f5", "#dce0e8"],
		},
		{
			id: "catppuccin-mocha",
			name: "catppuccin mocha",
			colors: ["#cdd6f4", "#1e1e2e", "#89b4fa", "#11111b"],
		},
		{
			id: "nord",
			name: "nord",
			colors: ["#d8dee9", "#2e3440", "#88c0d0", "#4c566a"],
		},
		{
			id: "rose-pine",
			name: "rosé pine",
			colors: ["#e0def4", "#191724", "#eb6f92", "#26233a"],
		},
	];
</script>

<div class="section-header">
	<div>
		<h2 class="section-title">appearance</h2>
		<p class="section-description">
			select a color scheme that suits your style.
		</p>
	</div>
	<button
		on:click={() => themeStore.toggle()}
		class="mode-toggle"
		aria-label="toggle light/dark mode"
	>
		{#if $themeStore === themes.dark}
			<Sun size={18} />
		{:else}
			<Moon size={18} />
		{/if}
	</button>
</div>

<div class="theme-grid">
	{#each availableThemes as theme (theme.id)}
		<button
			class="theme-card"
			class:active={$themeStore === theme.id}
			on:click={() => themeStore.set(theme.id)}
		>
			<div class="theme-palette">
				{#each theme.colors as color}
					<div class="palette-color" style="background-color: {color};"></div>
				{/each}
			</div>
			<div class="theme-name">
				{theme.name}
				{#if $themeStore === theme.id}
					<CheckCircle2 size={16} class="active-check" />
				{/if}
			</div>
		</button>
	{/each}
</div>

<style>
	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
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
	}
	.mode-toggle {
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-md);
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
		flex-shrink: 0;
	}
	.mode-toggle:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
		border-color: var(--border-secondary);
	}
	.theme-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: var(--spacing-lg);
	}
	.theme-card {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		overflow: hidden;
		transition: all 0.2s ease-in-out;
		cursor: pointer;
	}
	.theme-card:hover {
		transform: translateY(-4px);
		box-shadow: var(--shadow-md);
		border-color: var(--border-secondary);
	}
	.theme-card.active {
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 2px var(--accent-primary);
	}
	.theme-palette {
		display: flex;
		height: 100px;
	}
	.palette-color {
		flex: 1;
	}
	.theme-name {
		padding: var(--spacing-md);
		font-weight: 600;
		color: var(--text-primary);
		text-transform: capitalize;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.active-check {
		color: var(--accent-primary);
	}
</style>
