<script>
	import { theme as themeStore, themes } from "$lib/theme.js";
	import { appearance, updateAppearance } from "$lib/stores/settings.js";
	import { CheckCircle2, Sun, Moon } from "lucide-svelte";

	const availableThemes = [
		{
			id: themes.dark,
			name: "Neko Dark",
			colors: ["#e4e4e4", "#0c0c0c", "#8a8a8a", "#1c1c1c"],
			description: "Default dark theme with minimal contrast"
		},
		{
			id: themes.light,
			name: "Neko Light", 
			colors: ["#0c0c0c", "#ffffff", "#6c6c6c", "#f5f5f5"],
			description: "True black/white inverse of dark mode"
		},
		{
			id: themes.noir256,
			name: "Noir256",
			colors: ["#bcbcbc", "#000000", "#ff0000", "#121212"],
			description: "Authentic noir256 vim theme with red accents"
		},
		{
			id: themes.white,
			name: "Pure White",
			colors: ["#000000", "#ffffff", "#333333", "#f8f8f8"],
			description: "Bright white theme with maximum contrast"
		},
		{
			id: themes.tokyonight,
			name: "Tokyo Night",
			colors: ["#c0caf5", "#1a1b26", "#7aa2f7", "#24283b"],
			description: "Popular nvim theme with blue/purple accents"
		},
		{
			id: themes.tokyodark,
			name: "Tokyo Dark",
			colors: ["#A0A8CD", "#11121D", "#7199EE", "#1A1B2A"],
			description: "Deep, muted variant of Tokyo Night theme"
		},
		{
			id: themes.catppuccin_mocha,
			name: "Catppuccin Mocha",
			colors: ["#cdd6f4", "#1e1e2e", "#89b4fa", "#313244"],
			description: "Soothing pastel theme with warm colors"
		},
		{
			id: themes.catppuccin_latte,
			name: "Catppuccin Latte",
			colors: ["#4c4f69", "#eff1f5", "#1e66f5", "#dce0e8"],
			description: "Light variant of Catppuccin theme"
		},
		{
			id: themes.nord,
			name: "Nord",
			colors: ["#d8dee9", "#2e3440", "#88c0d0", "#434c5e"],
			description: "Arctic-inspired color palette"
		},
		{
			id: themes.rosepine,
			name: "Rosé Pine",
			colors: ["#e0def4", "#191724", "#c4a7e7", "#26233a"],
			description: "All natural pine, faux fur and a bit of soho vibes"
		},
		{
			id: themes.gruvbox_dark,
			name: "Gruvbox Dark",
			colors: ["#ebdbb2", "#282828", "#fabd2f", "#3c3836"],
			description: "Retro groove color scheme with warm tones"
		},
		{
			id: themes.gruvbox_light,
			name: "Gruvbox Light",
			colors: ["#3c3836", "#fbf1c7", "#b57614", "#ebdbb2"],
			description: "Light variant of the retro groove theme"
		},
		{
			id: themes.onedark,
			name: "One Dark",
			colors: ["#abb2bf", "#282c34", "#61afef", "#2c313c"],
			description: "Atom's iconic One Dark theme"
		},
		{
			id: themes.dracula,
			name: "Dracula",
			colors: ["#f8f8f2", "#282a36", "#ff79c6", "#44475a"],
			description: "Dark theme with vibrant pink and purple"
		},
		{
			id: themes.kawaii_light,
			name: "Kawaii Light",
			colors: ["#7a4f5a", "#fff5f7", "#ff9bb5", "#f8d7dd"],
			description: "Cute pastel theme with kawaii aesthetics :3"
		},
		{
			id: themes.kawaii_dark,
			name: "Kawaii Dark",
			colors: ["#f5d0f7", "#2d1b2e", "#e892f0", "#4d3b4e"],
			description: "Adorable dark theme with kawaii vibes >_<"
		}
	];

	function handleThemeChange(themeId) {
		themeStore.set(themeId);
		updateAppearance({ theme: themeId });
	}

</script>

<div class="section-header">
	<div>
		<h2 class="section-title">appearance</h2>
		<p class="section-description">
			Choose from popular themes including nvim-inspired color schemes.
		</p>
	</div>
		<button
			on:click={() => {
				const newTheme = $themeStore === themes.dark ? themes.light : themes.dark;
				handleThemeChange(newTheme);
			}}
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
			on:click={() => handleThemeChange(theme.id)}
		>
			<div class="theme-palette">
				{#each theme.colors as color}
					<div class="palette-color" style="background-color: {color};"></div>
				{/each}
			</div>
			<div class="theme-info">
				<div class="theme-name">
					{theme.name}
					{#if $themeStore === theme.id}
						<CheckCircle2 size={16} class="active-check" />
					{/if}
				</div>
				<div class="theme-description">
					{theme.description}
				</div>
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
	.theme-info {
		padding: var(--spacing-md);
	}
	.theme-name {
		font-weight: 600;
		color: var(--text-primary);
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--spacing-xs);
	}
	.theme-description {
		font-size: var(--font-size-sm);
		color: var(--text-secondary);
		line-height: var(--line-height-tight);
	}
</style>
