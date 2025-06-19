<script>
	import { Brain, ChevronDown } from "lucide-svelte";
	
	export let reasoningEffort = "medium";
	export let enabled = false;
	export let disabled = false;
	
	let showDropdown = false;
	
	const efforts = [
		{ value: "low", label: "Low", description: "Quick thinking" },
		{ value: "medium", label: "Medium", description: "Balanced reasoning" },
		{ value: "high", label: "High", description: "Deep analysis" }
	];
	
	function selectEffort(effort) {
		reasoningEffort = effort;
		showDropdown = false;
	}
	
	function toggleDropdown() {
		if (!disabled) {
			showDropdown = !showDropdown;
		}
	}
	
	function clickOutside(node, handler) {
		const handleClick = (event) => {
			if (node && !node.contains(event.target) && !event.defaultPrevented) {
				handler();
			}
		};
		document.addEventListener("click", handleClick, true);
		return {
			destroy() {
				document.removeEventListener("click", handleClick, true);
			},
		};
	}
</script>

<div class="reasoning-selector" class:enabled class:disabled>
	<button
		class="reasoning-toggle"
		class:active={enabled}
		class:disabled
		on:click={() => { enabled = !enabled; }}
		title="Enable reasoning mode"
	>
		<Brain size={18} />
	</button>
	
	{#if enabled}
		<div class="reasoning-effort" use:clickOutside={() => (showDropdown = false)}>
			<button
				class="effort-button"
				class:disabled
				on:click={toggleDropdown}
				title="Select reasoning effort"
			>
				<span class="effort-label">
					{efforts.find(e => e.value === reasoningEffort)?.label || "Medium"}
				</span>
				<div class="chevron" class:rotated={showDropdown}>
					<ChevronDown size={14} />
				</div>
			</button>
			
			{#if showDropdown}
				<div class="effort-dropdown">
					{#each efforts as effort}
						<button
							class="effort-option"
							class:selected={effort.value === reasoningEffort}
							on:click={() => selectEffort(effort.value)}
						>
							<div class="effort-info">
								<span class="effort-name">{effort.label}</span>
								<span class="effort-desc">{effort.description}</span>
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.reasoning-selector {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}
	
	.reasoning-toggle {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-xl);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: all var(--transition-fast);
	}
	
	.reasoning-toggle:hover:not(.disabled) {
		background-color: var(--bg-tertiary);
		color: var(--text-primary);
	}
	
	.reasoning-toggle.active {
		background-color: var(--accent-primary-muted);
		color: var(--accent-primary);
	}
	
	.reasoning-toggle.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.reasoning-effort {
		position: relative;
		display: flex;
		align-items: center;
	}
	
	.effort-button {
		background: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-md);
		padding: var(--spacing-xs) var(--spacing-sm);
		color: var(--text-primary);
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-xs);
		font-weight: 500;
		transition: all var(--transition-fast);
		min-width: 70px;
		height: 2rem;
	}
	
	.effort-button:hover:not(.disabled) {
		background-color: var(--bg-tertiary);
		border-color: var(--border-secondary);
	}
	
	.effort-button.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.effort-label {
		flex: 1;
		text-align: left;
	}
	
	.chevron {
		transition: transform var(--transition-fast);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	
	.chevron.rotated {
		transform: rotate(180deg);
	}
	
	.effort-dropdown {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		z-index: 1000;
		min-width: 140px;
		overflow: hidden;
		animation: dropdown-appear 0.15s ease-out;
	}
	
	.effort-option {
		width: 100%;
		background: transparent;
		border: none;
		padding: var(--spacing-sm);
		cursor: pointer;
		text-align: left;
		transition: background-color var(--transition-fast);
		border-bottom: 1px solid var(--border-primary);
	}
	
	.effort-option:last-child {
		border-bottom: none;
	}
	
	.effort-option:hover {
		background-color: var(--bg-tertiary);
	}
	
	.effort-option.selected {
		background-color: var(--accent-primary-muted);
		color: var(--accent-primary);
	}
	
	.effort-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	
	.effort-name {
		font-size: var(--font-size-xs);
		font-weight: 600;
		color: var(--text-primary);
	}
	
	.effort-option.selected .effort-name {
		color: var(--accent-primary);
	}
	
	.effort-desc {
		font-size: var(--font-size-xs);
		color: var(--text-muted);
		line-height: 1.2;
	}
	
	@keyframes dropdown-appear {
		from {
			opacity: 0;
			transform: translateY(-4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>