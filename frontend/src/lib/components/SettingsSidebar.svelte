<script>
	import { createEventDispatcher } from "svelte";
	import {
		Key,
		MessageSquare,
		Keyboard,
		Palette,
		ArrowLeft,
		User,
		Bot,
	} from "lucide-svelte";

	export let activeSection = "api-keys";

	// this is fine. using the dispatcher is the correct svelte pattern
	// for child-to-parent communication without prop drilling.
	const dispatch = createEventDispatcher();

	// if this component were to be more reusable, you'd pass this
	// array in as a prop. for a dedicated settings view, this is acceptable.
	const sections = [
		{ id: "general", label: "general", icon: User },
		{ id: "api-keys", label: "api keys", icon: Key },
		{ id: "models", label: "models", icon: Bot },
		{ id: "prompts", label: "system prompts", icon: MessageSquare },
		{ id: "shortcuts", label: "keyboard shortcuts", icon: Keyboard },
		{ id: "appearance", label: "appearance", icon: Palette },
	];

	// no changes needed here. it does what it says on the tin.
	function selectSection(sectionId) {
		dispatch("select", sectionId);
	}
</script>

<!-- the overall structure is mostly the same, but we're unifying class names -->
<aside class="sidebar">
	<div class="sidebar-header">
		<div class="header-content">
			<!-- "Settings" is a proper noun here, but given the context of your
			     app title, we'll keep it lowercase for stylistic consistency. -->
			<h1 class="app-title font-mono">settings</h1>
		</div>
	</div>

	<div class="sidebar-content">
		<div class="list-container">
			<nav class="list-nav">
				{#each sections as section (section.id)}
					<!-- using a button is semantically correct for in-page actions -->
					<button
						class="list-item"
						class:active={activeSection === section.id}
						on:click={() => selectSection(section.id)}
						aria-label={section.label}
					>
						<svelte:component this={section.icon} size={16} />
						<span class="list-item-label">{section.label}</span>
					</button>
				{/each}
			</nav>
		</div>
	</div>

	<div class="sidebar-footer">
		<!-- this is an anchor tag, implying navigation. correct. -->
		<a class="footer-btn" href="/">
			<ArrowLeft size={18} />
			<span>back to chat</span>
		</a>
	</div>
</aside>

<style>
	/* this is the core of the refactor: unifying styles with the main sidebar.
	   we're abstracting common patterns into more generic class names.
	   this is less about aesthetics and more about architectural sanity. */

	.sidebar {
		display: flex;
		flex-direction: column;
		height: 100vh;
		width: 250px;
		background-color: var(--bg-secondary);
		border-right: 1px solid var(--border-primary);
		flex-shrink: 0;
	}

	.sidebar-header {
		padding: var(--spacing-md);
		flex-shrink: 0;
	}

	.header-content {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.app-title {
		font-size: var(--font-size-lg);
		font-weight: 700;
		color: var(--text-primary);
		letter-spacing: -0.025em;
		white-space: nowrap;
	}

	.sidebar-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.list-container {
		flex: 1;
		overflow-y: auto;
		padding: var(--spacing-md);
	}

	.list-nav {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}

	/* generic list item style. this replaces both .chat-item and .nav-item.
	   this is the DRY principle applied to css. you should do this in your
	   main sidebar as well. */
	.list-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		background-color: transparent;
		border: none;
		border-radius: var(--radius-lg);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
		text-align: left;
		width: 100%;
		font-size: var(--font-size-sm);
	}

	.list-item:focus-visible {
		outline: 2px solid var(--accent-primary);
		outline-offset: -2px;
	}

	/* no more janky translateX. just a clean background and color shift. */
	.list-item:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}

	.list-item.active {
		background-color: var(--interactive-selected);
		color: var(--text-primary);
		font-weight: 600;
	}

	.list-item-label {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.list-item.active .list-item-label {
		font-weight: 600;
	}

	.sidebar-footer {
		padding: var(--spacing-md);
		flex-shrink: 0;
	}

	/* this can also be a shared class */
	.footer-btn {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border-radius: var(--radius-lg);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
		font-weight: 500;
		text-decoration: none;
		background-color: transparent;
		border: none;
	}

	.footer-btn:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}
</style>
