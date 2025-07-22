<script>
	import { createEventDispatcher } from 'svelte';

	export let show = false;

	const dispatch = createEventDispatcher();

	function handleClick() {
		dispatch('scroll');
	}
</script>

{#if show}
	<button
		class="scroll-to-bottom-btn"
		on:click={handleClick}
		title="Scroll to bottom"
		aria-label="scroll-to-bottom"
	>
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
		>
			<path d="m7 13 5 5 5-5" />
			<path d="m7 6 5 5 5-5" />
		</svg>
	</button>
{/if}

<style>
	.scroll-to-bottom-btn {
		position: absolute;
		bottom: 80px;
		right: 50%;
		transform: translateX(50%);
		width: 40px;
		height: 40px;
		border-radius: 50%;
		border: 1px solid var(--border-color);
		background: var(--bg-primary);
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		transition: all 0.2s ease;
		z-index: 5;
	}

	.scroll-to-bottom-btn:hover {
		background: var(--bg-secondary);
		color: var(--text-primary);
		transform: translateX(50%) translateY(-2px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
	}
</style>