<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { Search, Clock, MessageSquare, Hash } from 'lucide-svelte';
	import { chats, setActiveChat } from '$lib/stores/chats.js';
	import { chatAPI } from '$lib/api/chats.js';
	
	export let isOpen = false;
	
	const dispatch = createEventDispatcher();
	
	let searchInput;
	let searchTerm = '';
	let selectedIndex = 0;
	let modal;
	let chatMessages = new Map();
	let isSearching = false;
	
	$: recentChats = $chats.slice(0, 5);
	let filteredChats = [];
	
	$: if (searchTerm.trim()) {
		searchChatsWithContent(searchTerm).then(results => {
			filteredChats = results;
		});
	} else {
		filteredChats = recentChats;
	}
	
	async function searchChatsWithContent(term) {
		if (!term.trim()) return [];
		
		isSearching = true;
		const results = [];
		
		const titleResults = $chats.filter(chat => 
			chat.title.toLowerCase().includes(term.toLowerCase()) ||
			chat.model.toLowerCase().includes(term.toLowerCase()) ||
			chat.provider.toLowerCase().includes(term.toLowerCase())
		);
		
		for (const chat of $chats) {
			let messages = chatMessages.get(chat.id);
			
			if (!messages) {
				try {
					messages = await chatAPI.getMessages(chat.id);
					chatMessages.set(chat.id, messages);
				} catch (error) {
					console.warn(`Failed to load messages for chat ${chat.id}:`, error);
					continue;
				}
			}
			
			const hasContentMatch = messages.some(msg => 
				msg.content.toLowerCase().includes(term.toLowerCase())
			);
			
			if (hasContentMatch && !titleResults.find(c => c.id === chat.id)) {
				results.push({ ...chat, matchType: 'content' });
			}
		}
		
		const titleMatches = titleResults.map(chat => ({ ...chat, matchType: 'title' }));
		const allResults = [...titleMatches, ...results];
		
		isSearching = false;
		return allResults.slice(0, 10);
	}
	
	function handleKeydown(event) {
		if (!isOpen) return;
		
		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				selectedIndex = Math.min(selectedIndex + 1, filteredChats.length - 1);
				break;
			case 'ArrowUp':
				event.preventDefault();
				selectedIndex = Math.max(selectedIndex - 1, 0);
				break;
			case 'Enter':
				event.preventDefault();
				if (filteredChats[selectedIndex]) {
					selectChat(filteredChats[selectedIndex]);
				}
				break;
			case 'Escape':
				event.preventDefault();
				close();
				break;
		}
	}
	
	async function selectChat(chat) {
		await setActiveChat(chat.id);
		close();
	}
	
	function close() {
		isOpen = false;
		searchTerm = '';
		selectedIndex = 0;
		dispatch('close');
	}
	
	function handleModalClick(event) {
		if (event.target === modal) {
			close();
		}
	}
	
	$: if (isOpen && searchInput) {
		searchInput.focus();
		selectedIndex = 0;
	}
	
	$: if (searchTerm !== undefined) {
		selectedIndex = 0;
	}
	
	onMount(() => {
		document.addEventListener('keydown', handleKeydown);
	});
	
	onDestroy(() => {
		document.removeEventListener('keydown', handleKeydown);
	});
	
	function highlightMatch(text, term) {
		if (!term.trim()) return text;
		
		const regex = new RegExp(`(${term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
		return text.replace(regex, '<mark>$1</mark>');
	}
	
</script>

{#if isOpen}
	<div class="modal-backdrop" bind:this={modal} on:click={handleModalClick} on:keydown role="dialog" tabindex="0">
		<div class="modal-content">
			<div class="search-header">
				<div class="search-container">
					<Search size={16} style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--text-tertiary); pointer-events: none; z-index: 2;" />
					<input
						bind:this={searchInput}
						bind:value={searchTerm}
						type="text"
						placeholder="Search chats and messages..."
						class="search-input"
						autocomplete="off"
						spellcheck="false"
					/>
				</div>
			</div>
			
			<div class="results-container">
				{#if !searchTerm.trim()}
					<div class="section">
						<div class="section-header">
							<span>Recent</span>
						</div>
						{#each recentChats as chat, index (chat.id)}
							<button
								class="result-item"
								class:selected={index === selectedIndex}
								on:click={() => selectChat(chat)}
							>
								<div class="result-content">
									<MessageSquare size={16} style="color: var(--text-tertiary); flex-shrink: 0;" />
									<div class="chat-info">
										<div class="chat-title">{chat.title}</div>
										<div class="chat-meta">{chat.model}</div>
									</div>
								</div>
							</button>
						{/each}
					</div>
				{:else}
					<div class="section">
						<div class="section-header">
							<span>Results {isSearching ? '(searching...)' : `(${filteredChats.length})`}</span>
						</div>
						{#if isSearching}
							<div class="loading">
								<div class="loading-text">Searching through messages...</div>
							</div>
						{:else}
							{#each filteredChats as chat, index (chat.id)}
								<button
									class="result-item"
									class:selected={index === selectedIndex}
									on:click={() => selectChat(chat)}
								>
									<div class="result-content">
										<MessageSquare size={16} style="color: var(--text-tertiary); flex-shrink: 0;" />
										<div class="chat-info">
											<div class="chat-title">
												{@html highlightMatch(chat.title, searchTerm)}
												{#if chat.matchType === 'content'}
													<span class="match-indicator">in messages</span>
												{/if}
											</div>
											<div class="chat-meta">{chat.model}</div>
										</div>
									</div>
								</button>
							{:else}
								<div class="no-results">
									<div class="no-results-text">No chats found</div>
									<div class="no-results-subtitle">Try a different search term</div>
								</div>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
			
			<div class="modal-footer">
				<div class="shortcuts">
					<kbd>↑↓</kbd> Navigate
					<kbd>↵</kbd> Select
					<kbd>Esc</kbd> Close
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(4px);
		z-index: 1000;
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: 15vh;
		animation: fadeIn 0.15s ease-out;
	}
	
	.modal-content {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		width: 100%;
		max-width: 600px;
		max-height: 70vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		animation: slideIn 0.15s ease-out;
	}
	
	.search-header {
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--border-primary);
	}
	
	.search-container {
		position: relative;
		display: flex;
		align-items: center;
	}
	
	
	.search-input {
		width: 100%;
		padding: 12px 12px 12px 20px;
		color: var(--text-primary);
		font-size: var(--font-size-md);
		outline: none;
		transition: border-color var(--transition-fast);
		border: 0;
		border-bottom: 1px solid var(--border-primary);
		border-radius: 0;

	}
	
	.search-input:focus {
		border-color: var(--accent-primary);
		border: 0;
		box-shadow: 0 !important;
	}

input:focus {
    outline: none;
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px rgba(135, 135, 135, 0);
}
	
	.section {
		padding: 0 var(--spacing-lg);
	}
	
	.section-header {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm) 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	
	.result-item {
		width: 100%;
		padding: var(--spacing-sm) var(--spacing-md);
		border: none;
		background: none;
		text-align: left;
		cursor: pointer;
		border-radius: var(--radius-lg);
		margin-bottom: var(--spacing-xs);
		transition: background-color var(--transition-fast);
	}
	
	.result-item:hover,
	.result-item.selected {
		background-color: var(--interactive-hover);
	}
	
	.result-item.selected {
		background-color: var(--accent-primary);
		color: white;
	}
	
	.result-content {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
	}
	
	
	.chat-info {
		flex: 1;
		min-width: 0;
	}
	
	.chat-title {
		font-weight: 500;
		color: var(--text-primary);
		margin-bottom: 2px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.result-item.selected .chat-title {
		color: white;
	}
	
	.chat-meta {
		font-size: var(--font-size-xs);
		color: var(--text-tertiary);
	}
	
	.result-item.selected .chat-meta {
		color: rgba(255, 255, 255, 0.8);
	}
	
	.no-results {
		text-align: center;
		padding: var(--spacing-xl);
		color: var(--text-secondary);
	}
	
	.no-results-text {
		font-weight: 500;
		margin-bottom: var(--spacing-xs);
	}
	
	.no-results-subtitle {
		font-size: var(--font-size-sm);
		color: var(--text-tertiary);
	}
	
	.loading {
		padding: var(--spacing-xl);
		text-align: center;
		color: var(--text-secondary);
	}
	
	.loading-text {
		font-size: var(--font-size-sm);
	}
	
	.match-indicator {
		display: inline-block;
		margin-left: var(--spacing-xs);
		padding: 2px 6px;
		background-color: var(--accent-primary);
		color: white;
		font-size: var(--font-size-xs);
		border-radius: var(--radius-sm);
		opacity: 0.8;
	}
	
	.modal-footer {
		padding: var(--spacing-md) var(--spacing-lg);
		border-top: 1px solid var(--border-primary);
		background-color: var(--bg-tertiary);
	}
	
	.shortcuts {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		font-size: var(--font-size-xs);
		color: var(--text-tertiary);
	}
	
	kbd {
		padding: 2px 6px;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
		font-size: var(--font-size-xs);
		color: var(--text-secondary);
	}
	
	:global(mark) {
		background-color: var(--accent-primary);
		color: white;
		padding: 1px 2px;
		border-radius: 2px;
	}
	
	@keyframes fadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}
	
	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(-20px) scale(0.95);
		}
		to {
			opacity: 1;
			transform: translateY(0) scale(1);
		}
	}
</style>
