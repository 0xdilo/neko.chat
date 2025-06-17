<script>
	import {
		PanelsTopLeft,
		Settings,
		Plus,
		Search,
		Pin,
		Trash2,
		LogOut,
		Sun,
		Moon,
		GitBranch
	} from "lucide-svelte";
	import { sidebarCollapsed, rightSidebarCollapsed } from "$lib/stores/ui.js";
	import { 
		chats, 
		activeChat, 
		createChat, 
		deleteChat, 
		setActiveChat,
		loadChats,
		updateChat,
		switchToBranch
	} from "$lib/stores/chats.js";
	import { isAuthenticated, logout } from "$lib/stores/auth.js";
	import { theme, themes } from "$lib/theme.js";
	import { onMount, onDestroy } from "svelte";

	let searchTerm = "";
	let searchInput; // for bind:this
	let selectedIndex = 0;
	let isNavigationMode = false;

	// Filter chats based on search and pin status
	$: filteredPinnedChats = $chats
		.filter((c) => c.pinned && c.title.toLowerCase().includes(searchTerm.toLowerCase()));
	
	$: filteredUnpinnedChats = $chats
		.filter((c) => !c.pinned && c.title.toLowerCase().includes(searchTerm.toLowerCase()));

	// Create list of all navigable items for arrow key navigation
	$: navigationItems = [
		{ type: 'new-chat', id: 'new-chat' },
		...filteredPinnedChats.map(chat => ({ type: 'chat', id: chat.id })),
		...filteredUnpinnedChats.map(chat => ({ type: 'chat', id: chat.id })),
		{ type: 'settings', id: 'settings' },
		{ type: 'logout', id: 'logout' },
		{ type: 'theme-toggle', id: 'theme-toggle' }
	];

	// Listen for sidebar toggle events to enable navigation mode
	function handleSidebarToggle() {
		console.log('Sidebar toggle event received, current collapsed state:', $sidebarCollapsed);
		// Since the event is fired before the state changes, we need to check the opposite
		if ($sidebarCollapsed) {
			// Sidebar is currently collapsed, will be opened
			console.log('Activating navigation mode');
			isNavigationMode = true;
			selectedIndex = 0;
		} else {
			// Sidebar is currently open, will be collapsed
			console.log('Deactivating navigation mode');
			isNavigationMode = false;
		}
	}

	// Watch for sidebar state changes directly
	$: {
		console.log('Sidebar state changed - collapsed:', $sidebarCollapsed);
		if (!$sidebarCollapsed) {
			// Sidebar just opened
			console.log('Sidebar opened, activating navigation mode');
			isNavigationMode = true;
			selectedIndex = 0;
		} else {
			// Sidebar just closed
			console.log('Sidebar closed, deactivating navigation mode');
			isNavigationMode = false;
		}
	}

	onMount(async () => {
		if ($isAuthenticated) {
			await loadChats();
		}
		theme.init();

		if (typeof window !== "undefined") {
			window.addEventListener("toggle-sidebar", handleSidebarToggle);
			document.addEventListener("keydown", handleSidebarKeydown);
		}
	});

	onDestroy(() => {
		if (typeof window !== "undefined") {
			window.removeEventListener("toggle-sidebar", handleSidebarToggle);
			document.removeEventListener("keydown", handleSidebarKeydown);
		}
	});

	function toggleSidebar() {
		sidebarCollapsed.update((v) => {
			const newValue = !v;
			// If opening left sidebar, close right sidebar
			if (!newValue) {
				rightSidebarCollapsed.set(true);
			}
			return newValue;
		});
	}

	async function selectChat(chatId) {
		await switchToBranch(chatId);
	}

	async function handleNewChat() {
		await createChat({ title: 'New Chat' });
	}

	async function handleDeleteChat(chatId, e) {
		e.stopPropagation();
		await deleteChat(chatId);
	}
	
	async function handleTogglePin(chatId, isPinned, e) {
		e.stopPropagation();
		await updateChat(chatId, { pinned: !isPinned });
	}

	async function handleLogout() {
		await logout();
	}

	function handleKeydown(event) {
		if ((event.metaKey || event.ctrlKey) && event.key === "k") {
			event.preventDefault();
			searchInput?.focus();
		}
	}

	function handleSidebarKeydown(event) {
		// Only handle keys when sidebar is open and in navigation mode
		if ($sidebarCollapsed || !isNavigationMode) {
			return;
		}

		// Don't interfere when typing in search
		if (document.activeElement === searchInput) {
			return;
		}

		// Don't interfere when typing in chat input or if model selector is open
		const chatInput = document.getElementById('chat-input');
		const modelSelectorOpen = document.querySelector('.model-selector-dropdown');
		
		if (document.activeElement === chatInput || modelSelectorOpen) {
			return;
		}

		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				selectedIndex = (selectedIndex + 1) % navigationItems.length;
				break;
			case 'ArrowUp':
				event.preventDefault();
				selectedIndex = selectedIndex === 0 ? navigationItems.length - 1 : selectedIndex - 1;
				break;
			case 'Enter':
				event.preventDefault();
				handleNavigationItemSelect();
				break;
			case 'Escape':
				event.preventDefault();
				isNavigationMode = false;
				toggleSidebar();
				break;
		}
	}

	function handleNavigationItemSelect() {
		const item = navigationItems[selectedIndex];
		if (!item) return;

		switch (item.type) {
			case 'new-chat':
				handleNewChat();
				break;
			case 'chat':
				selectChat(item.id);
				break;
			case 'settings':
				window.location.href = '/settings';
				break;
			case 'logout':
				handleLogout();
				break;
			case 'theme-toggle':
				theme.toggle();
				break;
		}
	}

	function handleChatItemKeyDown(event, chatId) {
		if (event.key === "enter" || event.key === " ") {
			event.preventDefault();
			selectChat(chatId);
		}
	}

	function isItemSelected(type, id) {
		if (!isNavigationMode) return false;
		const currentItem = navigationItems[selectedIndex];
		return currentItem && currentItem.type === type && currentItem.id === id;
	}

	function formatChatTitle(chat) {
		// Check both the database flag and legacy title prefix for backward compatibility
		const isBranchFromFlag = chat.isBranch || chat.is_branch || false;
		console.log("penis")
		console.log(isBranchFromFlag)
		const isBranchFromTitle = chat.title.startsWith("Branch from: ");
		
		return {
			isBranch: isBranchFromFlag || isBranchFromTitle,
			displayTitle: isBranchFromTitle ? chat.title.replace("Branch from: ", "") : chat.title
		};
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="sidebar-container" class:collapsed={$sidebarCollapsed}>
	<!-- collapsed state: floating action buttons -->
	<div class="sidebar-collapsed" class:visible={$sidebarCollapsed}>
		<button
			class="collapse-btn"
			on:click={toggleSidebar}
			aria-label="expand sidebar"
		>
			<PanelsTopLeft />
		</button>
		<button
			class="action-btn"
			aria-label="search"
			on:click={() => searchInput?.focus()}
		>
			<Search />
		</button>
		<button class="action-btn" aria-label="new chat">
			<Plus />
		</button>
	</div>

	<!-- expanded state: full sidebar -->
	<aside class="sidebar">
		<div class="sidebar-header">
			<div class="header-content">
				<h1 class="app-title font-mono">neko.chat</h1>
				<button class="collapse-btn" on:click={toggleSidebar}>
					<PanelsTopLeft />
				</button>
			</div>
		</div>

		<div class="sidebar-content">
			<div class="actions-container">
				<button 
					class="new-chat-btn" 
					class:keyboard-selected={isNavigationMode && navigationItems[selectedIndex]?.type === 'new-chat'}
					aria-label="new chat" 
					on:click={handleNewChat}
				>
					<Plus size={18} />
					<span>new chat</span>
				</button>
				<div class="search-container">
					<Search class="search-icon" size={16} />
					<input
						type="text"
						placeholder="search... (Ctrl+k)"
						class="search-input"
						bind:value={searchTerm}
						bind:this={searchInput}
					/>
				</div>
			</div>

			<div class="chat-list">
				{#if filteredPinnedChats.length > 0}
					<h2 class="section-title">pinned</h2>
					<nav class="chat-nav">
						{#each filteredPinnedChats as chat (chat.id)}
							{@const titleInfo = formatChatTitle(chat)}
							<div
								class="chat-item"
								class:active={$activeChat === chat.id}
								class:keyboard-selected={isItemSelected('chat', chat.id)}
								role="button"
								tabindex="0"
								on:click={() => selectChat(chat.id)}
								on:keydown={(e) => handleChatItemKeyDown(e, chat.id)}
								aria-label={`chat: ${chat.title}`}
							>
								<div class="chat-item-content">
									<div class="chat-title-container">
										{#if titleInfo.isBranch}
											<GitBranch size={14} class="branch-icon" />
										{/if}
										<span class="chat-title">{titleInfo.displayTitle}</span>
									</div>
									<div class="chat-actions-hover">
										<button
											class="action-icon is-pinned"
											aria-label="unpin chat"
											on:click={(e) => handleTogglePin(chat.id, true, e)}
										>
											<Pin size={14} />
										</button>
										<button
											class="action-icon"
											aria-label="delete chat"
											on:click={(e) => handleDeleteChat(chat.id, e)}
										>
											<Trash2 size={14} />
										</button>
									</div>
								</div>
							</div>
						{/each}
					</nav>
				{/if}

				<h2 class="section-title">conversations</h2>
				<nav class="chat-nav">
					{#each filteredUnpinnedChats as chat (chat.id)}
						{@const titleInfo = formatChatTitle(chat)}
						<div
							class="chat-item"
							class:active={$activeChat === chat.id}
							class:keyboard-selected={isItemSelected('chat', chat.id)}
							role="button"
							tabindex="0"
							on:click={() => selectChat(chat.id)}
							on:keydown={(e) => handleChatItemKeyDown(e, chat.id)}
							aria-label={`chat: ${chat.title}`}
						>
							<div class="chat-item-content">
								<div class="chat-title-container">
									{#if titleInfo.isBranch}
										<GitBranch size={14} class="branch-icon" />
									{/if}
									<span class="chat-title">{titleInfo.displayTitle}</span>
								</div>
								<div class="chat-actions-hover">
									<button
										class="action-icon"
										aria-label="pin chat"
										on:click={(e) => handleTogglePin(chat.id, false, e)}
									>
										<Pin size={14} />
									</button>
									<button
										class="action-icon"
										aria-label="delete chat"
										on:click={(e) => handleDeleteChat(chat.id, e)}
									>
										<Trash2 size={14} />
									</button>
								</div>
							</div>
						</div>
					{:else}
						{#if filteredPinnedChats.length === 0}
							<div class="empty-state">
								<p>No chats yet</p>
								<small>Create your first chat to get started!</small>
							</div>
						{/if}
					{/each}
				</nav>
			</div>
		</div>

		<div class="sidebar-footer">
			<a 
				class="settings-btn" 
				class:keyboard-selected={isItemSelected('settings', 'settings')}
				aria-label="settings" 
				href="/settings"
			>
				<Settings size={18} />
				<span>settings</span>
			</a>
			<button 
				class="settings-btn" 
				class:keyboard-selected={isItemSelected('logout', 'logout')}
				aria-label="logout" 
				on:click={handleLogout}
			>
				<LogOut size={18} />
				<span>logout</span>
			</button>
				<div class="theme-toggle-container">
					<button
						on:click={() => theme.toggle()}
						class="theme-toggle-btn"
						class:keyboard-selected={isItemSelected('theme-toggle', 'theme-toggle')}
						aria-label="toggle light/dark mode"
					>
						{#if $theme === themes.dark}
							<Sun size={18} />
							<span>light mode</span>
						{:else}
							<Moon size={18} />
							<span>dark mode</span>
						{/if}
					</button>
				</div>
		</div>
	</aside>
</div>

<style>
	.sidebar-container {
		position: relative;
		flex-shrink: 0;
		width: 250px;
		transition: width var(--transition-normal);
		overflow: hidden;
	}

	.sidebar-container.collapsed {
		width: 0;
	}

	.sidebar {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background-color: var(--bg-secondary);
		border-right: 1px solid var(--border-primary);
		width: 250px;
		position: relative;
	}

	.sidebar-collapsed {
		position: fixed;
		top: var(--spacing-md);
		left: var(--spacing-md);
		z-index: 1000;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		transform: translateX(-100%);
		opacity: 0;
		pointer-events: none;
		transition: transform var(--transition-normal),
			opacity var(--transition-normal);
	}

	.sidebar-collapsed.visible {
		transform: translateX(0);
		opacity: 1;
		pointer-events: auto;
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

	.collapse-btn,
	.action-btn {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		cursor: pointer;
		padding: var(--spacing-sm);
		transition: all var(--transition-fast);
		border-radius: var(--radius-lg);
		color: var(--text-primary);
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		transform: scale(1);
	}

	.collapse-btn:hover,
	.action-btn:hover {
		background-color: var(--interactive-hover);
		color: var(--accent-primary);
		transform: scale(1.05);
	}

	.collapse-btn:active,
	.action-btn:active {
		transform: scale(0.95);
	}

	.sidebar-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.actions-container {
		padding: var(--spacing-md);
		padding-top: 0;
		border-bottom: 1px solid var(--border-primary);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.new-chat-btn {
		width: 100%;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		color: var(--text-primary);
		font-weight: 500;
		cursor: pointer;
		transition: all var(--transition-fast);
		justify-content: center;
		transform: scale(1);
	}

	.new-chat-btn:hover {
		background-color: var(--interactive-hover);
		border-color: var(--border-secondary);
		transform: scale(1.02);
	}

	.new-chat-btn:active {
		transform: scale(0.98);
	}

	.new-chat-btn.keyboard-selected {
		background-color: var(--accent-primary);
		border-color: var(--accent-primary);
		color: white;
	}

	.search-container {
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-icon {
		position: absolute;
		left: 12px;
		top: 50%;
		transform: translateY(-50%);
		color: var(--text-tertiary);
		pointer-events: none;
		z-index: 2;
	}

	.search-input {
		width: 100%;
          border: 0px;
          border-radius: 0px;
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		transition: border-color var(--transition-fast);
		box-shadow: none;
	}

	.search-input:focus {
          border: 0px;
	}

	.chat-list {
		flex: 1;
		overflow-y: auto;
		padding: var(--spacing-md);
	}

	.section-title {
		font-size: var(--font-size-xs);
		font-weight: 600;
		color: var(--text-secondary);
		margin-bottom: var(--spacing-md);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.chat-nav {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		margin-bottom: 20px;
	}

	.chat-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-md);
		background-color: transparent;
		border: none;
		border-radius: var(--radius-lg);
		color: var(--text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
		text-align: left;
		min-height: 2rem;
		position: relative;
		font-size: var(--font-size-xs);
	}

	.chat-item:focus-visible {
		outline: 2px solid var(--accent-primary);
		outline-offset: -2px;
	}

	.chat-item:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}

	.chat-item.active {
		background-color: var(--interactive-selected);
		color: var(--text-primary);
	}

	.chat-item.keyboard-selected {
		background-color: var(--accent-primary);
		color: white;
		border: 1px solid var(--accent-primary);
	}

	.chat-item-content {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		gap: var(--spacing-sm);
	}

	.chat-title-container {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		flex: 1;
		min-width: 0; /* Allow shrinking */
	}

	.chat-title {
		font-weight: 500;
		font-size: var(--font-size-xs);
		margin-bottom: 2px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
	}

	:global(.branch-icon) {
		color: var(--accent-primary);
		flex-shrink: 0;
	}

	.chat-actions-hover {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		opacity: 0;
		transition: opacity var(--transition-fast);
		pointer-events: none;
	}

	.chat-item:hover .chat-actions-hover,
	.chat-item:focus-within .chat-actions-hover {
		opacity: 1;
		pointer-events: auto;
	}

	.action-icon {
		background: none;
		border: none;
		padding: var(--spacing-xs);
		border-radius: var(--radius-md);
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background-color var(--transition-fast),
			color var(--transition-fast);
	}

	.action-icon:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
	}

	.is-pinned {
		color: var(--accent-primary);
		fill: var(--accent-primary);
	}

	.sidebar-footer {
		padding: var(--spacing-md);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		flex-shrink: 0;
	}

	.settings-btn {
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
		font-weight: 500;
		transform: scale(1);
	}

	.settings-btn:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
		transform: scale(1.02);
	}

	.settings-btn:active {
		transform: scale(0.98);
	}

	.settings-btn.keyboard-selected {
		background-color: var(--accent-primary);
		color: white;
	}

	.theme-toggle-container {
		margin-top: var(--spacing-sm);
		border-top: 1px solid var(--border-primary);
		padding-top: var(--spacing-md);
	}

	.theme-toggle-btn {
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
		font-weight: 500;
		transform: scale(1);
		width: 100%;
	}

	.theme-toggle-btn:hover {
		background-color: var(--interactive-hover);
		color: var(--text-primary);
		transform: scale(1.02);
	}

	.theme-toggle-btn:active {
		transform: scale(0.98);
	}

	.theme-toggle-btn.keyboard-selected {
		background-color: var(--accent-primary);
		color: white;
	}

	.empty-state {
		text-align: center;
		padding: var(--spacing-xl);
		color: var(--text-secondary);
	}

	.empty-state small {
		color: var(--text-muted);
		font-size: var(--font-size-xs);
		display: block;
		margin-top: var(--spacing-xs);
	}
</style>
