<script>
	import Sidebar from "./Sidebar.svelte";
	import RightSidebar from "./RightSidebar.svelte";
	import { sidebarCollapsed, rightSidebarCollapsed } from "$lib/stores/ui.js";
</script>

<div class="app-layout">
	<slot name="sidebar">
		<Sidebar />
	</slot>

	<main class="main-content" class:sidebar-collapsed={$sidebarCollapsed} class:right-sidebar-collapsed={$rightSidebarCollapsed}>
		<slot />
	</main>

	<slot name="rightSidebar">
		<RightSidebar />
	</slot>
</div>

<style>
	.app-layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
		background-color: var(--bg-primary);
	}

	.main-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		transition: margin-left var(--transition-normal);
		/* define a css variable for the chat content's max-width */
		--chat-max-width: 800px;
	}

	.main-content.sidebar-collapsed {
		/* increase the max-width when the sidebar is collapsed */
		--chat-max-width: 1100px;
	}

	.main-content.right-sidebar-collapsed {
		/* increase the max-width when the right sidebar is collapsed */
		--chat-max-width: 1100px;
	}

	.main-content.sidebar-collapsed.right-sidebar-collapsed {
		/* increase the max-width when both sidebars are collapsed */
		--chat-max-width: 1200px;
	}


	@media (max-width: 768px) {
		.app-layout {
			flex-direction: column;
		}
	}
</style>
