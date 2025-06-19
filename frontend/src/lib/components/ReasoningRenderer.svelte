<script>
	import { Brain, ChevronDown, ChevronUp } from "lucide-svelte";
	import MarkdownRenderer from "./MarkdownRenderer.svelte";
	
	export let content = '';
	
	let reasoning = '';
	let mainContent = '';
	let showReasoning = false;
	let reasoningExpanded = false;
	
	// Extract reasoning from content if markers are present
	$: {
		if (content && content.includes('__REASONING_START__') && content.includes('__REASONING_END__')) {
			const startIndex = content.indexOf('__REASONING_START__');
			const endIndex = content.indexOf('__REASONING_END__');
			
			if (startIndex !== -1 && endIndex !== -1) {
				reasoning = content.substring(startIndex + '__REASONING_START__'.length, endIndex).trim();
				mainContent = content.replace(/__REASONING_START__[\s\S]*?__REASONING_END__/, '').trim();
				showReasoning = true;
			} else {
				mainContent = content;
				showReasoning = false;
			}
		} else {
			mainContent = content;
			showReasoning = false;
		}
	}
	
	function toggleReasoning() {
		reasoningExpanded = !reasoningExpanded;
	}
</script>

<div class="reasoning-wrapper">
	{#if showReasoning && reasoning}
		<div class="reasoning-section">
			<button
				class="reasoning-toggle"
				on:click={toggleReasoning}
				aria-expanded={reasoningExpanded}
			>
				<div class="reasoning-icon">
					<Brain size={16} />
				</div>
				<span class="reasoning-label">Reasoning</span>
				<div class="chevron" class:expanded={reasoningExpanded}>
					{#if reasoningExpanded}
						<ChevronUp size={16} />
					{:else}
						<ChevronDown size={16} />
					{/if}
				</div>
			</button>
			
			{#if reasoningExpanded}
				<div class="reasoning-content">
					<MarkdownRenderer content={reasoning} />
				</div>
			{/if}
		</div>
	{/if}
	
	{#if mainContent && mainContent.trim()}
		<div class="main-content">
			<MarkdownRenderer content={mainContent} />
		</div>
	{/if}
</div>

<style>
	.reasoning-wrapper {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}
	
	.reasoning-section {
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		overflow: hidden;
		transition: all var(--transition-fast);
	}
	
	.reasoning-toggle {
		width: 100%;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: all var(--transition-fast);
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
		font-weight: 500;
	}
	
	.reasoning-toggle:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}
	
	.reasoning-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		background-color: var(--accent-primary-muted);
		color: var(--accent-primary);
		border-radius: var(--radius-sm);
		flex-shrink: 0;
	}
	
	.reasoning-label {
		flex: 1;
		text-align: left;
		font-weight: 600;
	}
	
	.chevron {
		display: flex;
		align-items: center;
		justify-content: center;
		transition: transform var(--transition-fast);
		color: var(--text-muted);
	}
	
	.chevron.expanded {
		transform: rotate(0deg);
	}
	
	.reasoning-content {
		padding: 0 var(--spacing-md) var(--spacing-md) var(--spacing-md);
		border-top: 1px solid var(--border-primary);
		background-color: var(--bg-secondary);
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-relaxed);
		animation: expand 0.2s ease-out;
	}
	
	@keyframes expand {
		from {
			opacity: 0;
			max-height: 0;
		}
		to {
			opacity: 1;
			max-height: 1000px;
		}
	}
	
	.main-content {
		/* Main content styling */
	}
</style>