<script>
  import { X } from "lucide-svelte";
  import ChatGraph from "./ChatGraph.svelte";
  import { chatTree, activeChat, switchToBranch } from "$lib/stores/chats.js";
  import { rightSidebarCollapsed, sidebarCollapsed } from "$lib/stores/ui.js";
  
  // Filter chat tree to only show branches related to the active chat
  $: activeChatTree = getActiveChatTree($chatTree, $activeChat);
  
  function getActiveChatTree(tree, activeChatId) {
    if (!tree || !activeChatId || Object.keys(tree).length === 0) {
      return {};
    }
    
    // Find the root chat for the active chat (either the active chat itself or its ancestor)
    function findRootChat(chatId) {
      const allChats = {};
      
      // Flatten the tree structure to get all chats
      function flattenTree(node) {
        allChats[node.id] = node;
        if (node.children) {
          node.children.forEach(child => flattenTree(child));
        }
      }
      
      Object.values(tree).forEach(rootNode => flattenTree(rootNode));
      
      // Find the chat and trace back to root
      let currentChat = allChats[chatId];
      if (!currentChat) return null;
      
      // Find the root by going up the parent chain
      while (currentChat.parentChatId && allChats[currentChat.parentChatId]) {
        currentChat = allChats[currentChat.parentChatId];
      }
      
      return currentChat;
    }
    
    const rootChat = findRootChat(activeChatId);
    if (!rootChat) return {};
    
    // Return the subtree starting from this root
    return { [rootChat.id]: rootChat };
  }
</script>

<div class="right-sidebar" class:collapsed={$rightSidebarCollapsed}>
  <div class="sidebar-header">
    <h3>Conversation Tree</h3>
    <button 
      class="close-button" 
      on:click={() => {
        rightSidebarCollapsed.set(true);
        sidebarCollapsed.set(false);
      }}
      title="Close sidebar"
    >
      <X size={16} />
    </button>
  </div>
  
  <div class="sidebar-content">
    <ChatGraph 
      chatTree={activeChatTree}
      activeChatId={$activeChat}
      on:nodeClick={(e) => switchToBranch(e.detail.chatId)}
    />
  </div>
</div>

<style>
  .right-sidebar {
    width: 300px;
    background-color: var(--bg-secondary);
    border-left: 1px solid var(--border-primary);
    transition: transform var(--transition-normal);
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .right-sidebar.collapsed {
    transform: translateX(100%);
    width: 0;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .sidebar-header h3 {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-button {
    padding: 0.25rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .close-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .sidebar-content {
    flex: 1;
    overflow: auto;
    padding: 1rem;
  }
</style>