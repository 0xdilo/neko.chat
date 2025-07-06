import { writable, derived, get } from "svelte/store";
import { browser } from "$app/environment";
import { goto } from "$app/navigation";
import { chatAPI } from "$lib/api/chats.js";
import { showError, showSuccess } from "./app.js";
import { rightSidebarCollapsed } from "./ui.js";
import { getFirstEnabledModel, lastUsedModel } from "./models.js";

function generateChatTitle(content) {
  if (!content || typeof content !== "string") {
    return "New Chat";
  }

  const cleanContent = content.trim().replace(/\s+/g, " ");

  if (cleanContent.length <= 50) {
    return cleanContent;
  }

  const sentences = cleanContent.split(/[.!?]+/);
  if (sentences[0] && sentences[0].length <= 50) {
    return sentences[0].trim();
  }

  // Find a good word boundary within 50 characters
  const words = cleanContent.split(" ");
  let title = "";
  for (const word of words) {
    if ((title + " " + word).length > 50) {
      break;
    }
    title += (title ? " " : "") + word;
  }

  return title || cleanContent.substring(0, 47) + "...";
}

// Chat store structure
export const chats = writable([]);
export const activeChat = writable(null);
export const activeChatMessages = writable([]);
export const isLoading = writable(false);
export const error = writable(null);

// Graph-based chat management
export const chatTree = writable({});
export const streamingChats = writable(new Set());
export const streamingMessages = writable({}); // Store streaming messages by chat ID

// Track if loadChats has been called to prevent multiple simultaneous calls
let loadChatsPromise = null;

// Load chats from API
export async function loadChats() {
  // If already loading, return the existing promise
  if (loadChatsPromise) {
    return loadChatsPromise;
  }

  loadChatsPromise = (async () => {
    isLoading.set(true);
    error.set(null);

    try {
      const chatList = await chatAPI.getChats();
      chats.set(chatList);
      buildChatTree(chatList);

      // Only set the most recent chat as active if no chat is currently active
      // This prevents overriding URL-based chat selection on page refresh
      const currentActiveChat = get(activeChat);
      if (chatList.length > 0 && !currentActiveChat) {
        const mostRecent = chatList[0]; // Already sorted by backend
        await setActiveChat(mostRecent.id, false, true);
      }
    } catch (err) {
      console.error("Failed to load chats:", err);
      error.set(err.message);
      showError("Failed to load chats");
    } finally {
      isLoading.set(false);
      loadChatsPromise = null; // Reset for future calls
    }
  })();

  return loadChatsPromise;
}

// Build tree structure from flat chat list
export function buildChatTree(chatList) {
  const tree = {};
  const chatMap = {};

  // Create a map for quick lookup
  chatList.forEach((chat) => {
    chatMap[chat.id] = { ...chat, children: [] };
  });

  // Build parent-child relationships
  chatList.forEach((chat) => {
    if (chat.parentChatId && chatMap[chat.parentChatId]) {
      chatMap[chat.parentChatId].children.push(chatMap[chat.id]);
    } else {
      // Root level chat
      tree[chat.id] = chatMap[chat.id];
    }
  });
  chatTree.set(tree);
}

// Optimized function to update tree incrementally
function updateChatInTree(chat, isDelete = false) {
  chatTree.update((currentTree) => {
    const newTree = { ...currentTree };
    
    if (isDelete) {
      // Remove chat and its children from tree
      delete newTree[chat.id];
      // Also remove from parent's children if it's a branch
      if (chat.parentChatId && newTree[chat.parentChatId]) {
        newTree[chat.parentChatId].children = newTree[chat.parentChatId].children.filter(
          child => child.id !== chat.id
        );
      }
    } else {
      // Add or update chat in tree
      const chatWithChildren = { ...chat, children: currentTree[chat.id]?.children || [] };
      
      if (chat.parentChatId && newTree[chat.parentChatId]) {
        // It's a branch - add to parent's children
        const parentChildren = newTree[chat.parentChatId].children || [];
        const existingIndex = parentChildren.findIndex(child => child.id === chat.id);
        if (existingIndex >= 0) {
          parentChildren[existingIndex] = chatWithChildren;
        } else {
          parentChildren.push(chatWithChildren);
        }
        newTree[chat.parentChatId].children = parentChildren;
      } else {
        // Root level chat
        newTree[chat.id] = chatWithChildren;
      }
    }
    
    return newTree;
  });
}

// Create a new chat
export async function createChat(chatData = {}) {
  isLoading.set(true);

  try {
    const defaultModel = getFirstEnabledModel();
    const lastModel = get(lastUsedModel);
    
    // Use last used model if available and no specific model provided
    const modelToUse = chatData.provider && chatData.model 
      ? { provider: chatData.provider, model: chatData.model }
      : lastModel || defaultModel;
    
    const newChat = await chatAPI.createChat({
      title: chatData.title || "New Chat",
      system_prompt:
        chatData.system_prompt || "You are a helpful AI assistant.",
      provider: modelToUse.provider,
      model: modelToUse.model,
      is_branch: chatData.is_branch || false,
      parent_chat_id: chatData.parent_chat_id,
      branch_point_message_id: chatData.branch_point_message_id,
    });

    chats.update((chatList) => {
      const updatedList = [newChat, ...chatList];
      // Use incremental update instead of full rebuild for single chat
      updateChatInTree(newChat);
      return updatedList;
    });
    await setActiveChat(newChat.id);
    showSuccess("Chat created successfully");
    return newChat;
  } catch (err) {
    console.error("Failed to create chat:", err);
    showError("Failed to create chat");
    throw err;
  } finally {
    isLoading.set(false);
  }
}

// Set active chat and load its messages
export async function setActiveChat(chatId, preserveMessages = false, updateUrl = true) {
  if (!chatId) {
    activeChat.set(null);
    activeChatMessages.set([]);
    if (updateUrl && browser) {
      goto('/');
    }
    return;
  }

  if (!preserveMessages) {
    activeChatMessages.set([]);
  }
  activeChat.set(chatId);

  // Update URL to reflect the active chat
  if (updateUrl && browser) {
    goto(`/chat/${chatId}`);
  }

  try {
    const messages = await chatAPI.getMessages(chatId);
    activeChatMessages.set(messages);
  } catch (err) {
    console.error("Failed to load messages:", err);
    showError("Failed to load messages");
    activeChatMessages.set([]);
  }
}

// Update chat
export async function updateChat(chatId, updates) {
  try {
    const updatedChat = await chatAPI.updateChat(chatId, updates);
    chats.update((chatList) =>
      chatList.map((chat) => (chat.id === chatId ? updatedChat : chat)),
    );
    return updatedChat;
  } catch (err) {
    console.error("Failed to update chat:", err);
    showError("Failed to update chat");
    throw err;
  }
}

// Delete chat
export async function deleteChat(chatId) {
  try {
    await chatAPI.deleteChat(chatId);

    // Get the chat being deleted for tree update
    const currentChats = get(chats);
    const deletedChat = currentChats.find(chat => chat.id === chatId);

    // Update chats store and tree incrementally
    chats.update((chatList) => {
      const updatedChatList = chatList.filter((chat) => chat.id !== chatId);
      // Use incremental update instead of full rebuild
      if (deletedChat) {
        updateChatInTree(deletedChat, true);
      }
      return updatedChatList;
    });

    // If the deleted chat was active, set the first available chat as active
    const currentActive = await new Promise((resolve) => {
      activeChat.subscribe((current) => resolve(current))();
    });

    if (currentActive === chatId) {
      const currentChats = await new Promise((resolve) => {
        chats.subscribe((chatList) => resolve(chatList))();
      });

      if (currentChats.length > 0) {
        await setActiveChat(currentChats[0].id);
      } else {
        // No chats left, navigate to root
        if (browser) {
          goto('/');
        }
        activeChat.set(null);
        activeChatMessages.set([]);
      }
    }

    showSuccess("Chat deleted successfully");
  } catch (err) {
    console.error("Failed to delete chat:", err);
    showError("Failed to delete chat");
    throw err;
  }
}

// Add message to active chat
export function addMessageToActiveChat(message) {
  activeChatMessages.update((messages) => [...messages, message]);
}

// Update message in active chat
export function updateMessageInActiveChat(messageId, updates) {
  activeChatMessages.update((messages) =>
    messages.map((msg) =>
      msg.id === messageId ? { ...msg, ...updates } : msg,
    ),
  );
}

// Delete message from active chat
export async function deleteMessage(messageId) {
  const currentChatId = get(activeChat);
  if (!currentChatId) {
    throw new Error("No active chat");
  }

  try {
    await chatAPI.deleteMessage(currentChatId, messageId);

    // Remove message from UI immediately
    activeChatMessages.update((messages) =>
      messages.filter((msg) => msg.id !== messageId),
    );

    showSuccess("Message deleted");
  } catch (err) {
    console.error("Failed to delete message:", err);
    showError("Failed to delete message");
    throw err;
  }
}

// Update streaming message content (accumulative)
export function updateStreamingMessage(messageId, chunk) {
  activeChatMessages.update((messages) =>
    messages.map((msg) =>
      msg.id === messageId
        ? { ...msg, content: (msg.content || "") + chunk }
        : msg,
    ),
  );
}

// Get current active chat
export const currentChat = derived(
  [chats, activeChat],
  ([$chats, $activeChat]) => {
    return $chats.find((chat) => chat.id === $activeChat) || null;
  },
);

// Search chats
export function searchChats(query) {
  return derived(chats, ($chats) => {
    if (!query.trim()) return $chats;

    const searchTerm = query.toLowerCase();
    return $chats.filter((chat) =>
      chat.title.toLowerCase().includes(searchTerm),
    );
  });
}

// Send message to active chat with streaming
export async function sendMessage(content, options = {}) {
  let currentChatId = await new Promise((resolve) => {
    activeChat.subscribe((chatId) => resolve(chatId))();
  });

  // Auto-create a new chat if none exists
  if (!currentChatId) {
    try {
      // Use the model from options or fallback to first enabled model
      const selectedModel = options.model
        ? { provider: options.provider, model: options.model }
        : getFirstEnabledModel();

      const newChat = await createChat({
        title: generateChatTitle(content),
        system_prompt: "You are a helpful AI assistant.",
        provider: selectedModel.provider,
        model: selectedModel.model,
        is_branch: false,
      });
      currentChatId = newChat.id;
    } catch (err) {
      console.error("Failed to auto-create chat:", err);
      throw new Error("Failed to create chat for message");
    }
  }

  // Check if this is the first user message and update title if needed
  const currentMessages = get(activeChatMessages);
  const hasUserMessages = currentMessages.some((msg) => msg.role === "user");
  const currentChatData = get(currentChat);

  if (
    !hasUserMessages &&
    currentChatData &&
    (currentChatData.title === "New Chat" ||
      currentChatData.title.includes("New Chat"))
  ) {
    try {
      const newTitle = generateChatTitle(content);
      await updateChat(currentChatId, { title: newTitle });
    } catch (err) {
      console.warn("Failed to update chat title:", err);
      // Don't block message sending if title update fails
    }
  }

  // Add user message immediately to UI
  const userMessage = {
    id: `user-${Date.now()}`,
    role: "user",
    content,
    created_at: new Date().toISOString(),
  };

  addMessageToActiveChat(userMessage);

  // Add assistant message placeholder for streaming
  const assistantMessageId = `assistant-${Date.now()}`;
  const assistantMessage = {
    id: assistantMessageId,
    role: "assistant",
    content: "",
    created_at: new Date().toISOString(),
    streaming: true,
  };

  addMessageToActiveChat(assistantMessage);

  // Add to global streaming state for regular chats too
  streamingChats.update((set) => new Set(set).add(currentChatId));
  streamingMessages.update((messages) => ({
    ...messages,
    [currentChatId]: {
      userMessage,
      assistantMessage,
      assistantMessageId,
      content: "",
    },
  }));

  // Throttle UI updates to reduce churn
  let lastUpdateTime = 0;
  const UPDATE_THROTTLE_MS = 50; // Update UI at most every 50ms
  let pendingContent = "";

  try {
    // Stream message response
    await chatAPI.streamMessage(currentChatId, content, {
      webSearch: options.webSearch,
      onStart: options.onStart,
      onChunk: (chunk, accumulatedContent) => {
        // Update global streaming state immediately
        streamingMessages.update((messages) => ({
          ...messages,
          [currentChatId]: {
            ...messages[currentChatId],
            content: accumulatedContent,
          },
        }));

        // Throttle UI updates
        pendingContent = accumulatedContent;
        const now = Date.now();
        if (now - lastUpdateTime >= UPDATE_THROTTLE_MS) {
          lastUpdateTime = now;
          updateMessageInActiveChat(assistantMessageId, {
            content: pendingContent,
            streaming: true,
          });
        }
      },
      onComplete: async () => {
        // Final update with any pending content
        if (pendingContent) {
          updateMessageInActiveChat(assistantMessageId, {
            content: pendingContent,
            streaming: false,
          });
        }

        // Clean up global streaming state
        streamingChats.update((set) => {
          const newSet = new Set(set);
          newSet.delete(currentChatId);
          return newSet;
        });

        streamingMessages.update((messages) => {
          const updated = { ...messages };
          delete updated[currentChatId];
          return updated;
        });

        // Don't reload messages immediately - let websocket handle the final message update
        // The ContentSaver in the backend will send a websocket message with the saved message
      },
      onError: (error) => {
        console.error("Streaming error:", error);
        updateMessageInActiveChat(assistantMessageId, {
          content: "Error: Failed to get response",
          streaming: false,
          error: true,
        });

        // Clean up global streaming state on error
        streamingChats.update((set) => {
          const newSet = new Set(set);
          newSet.delete(currentChatId);
          return newSet;
        });

        streamingMessages.update((messages) => {
          const updated = { ...messages };
          delete updated[currentChatId];
          return updated;
        });

        showError("Failed to get response");
      },
      ...options,
    });

    return { success: true };
  } catch (err) {
    console.error("Failed to send message:", err);
    
    // If stream was aborted by user, just mark as completed without error
    if (err.name === 'AbortError') {
      // The message should already have the partial content from onChunk calls
      // Just mark it as not streaming anymore
      updateMessageInActiveChat(assistantMessageId, {
        streaming: false,
      });
      
      // Clean up global streaming state
      streamingChats.update((set) => {
        const newSet = new Set(set);
        newSet.delete(currentChatId);
        return newSet;
      });

      streamingMessages.update((messages) => {
        const updated = { ...messages };
        delete updated[currentChatId];
        return updated;
      });
      
      // Don't reload messages from database on abort - keep the UI state with partial content
      // Return success to indicate no error should be shown
      return { success: true, aborted: true };
    }
    
    // For other errors, show error message
    updateMessageInActiveChat(assistantMessageId, {
      content: "Error: Failed to send message",
      streaming: false,
      error: true,
    });
    showError("Failed to send message");
    throw err;
  }
}

// Send parallel message to multiple models
export async function sendParallelMessage(content, models, options = {}) {
  let currentChatId = await new Promise((resolve) => {
    activeChat.subscribe((chatId) => resolve(chatId))();
  });

  // Auto-create a new chat if none exists
  if (!currentChatId) {
    try {
      const defaultModel = getFirstEnabledModel();
      const newChat = await createChat({
        title: generateChatTitle(content),
        system_prompt: "You are a helpful AI assistant.",
        provider: defaultModel.provider,
        model: defaultModel.model,
        is_branch: false,
      });
      currentChatId = newChat.id;
    } catch (err) {
      console.error("Failed to auto-create chat for parallel message:", err);
      throw new Error("Failed to create chat for parallel message");
    }
  }

  if (!models || models.length === 0) {
    throw new Error("No models selected");
  }

  // Check if we need to update the parent chat title
  const currentMessages = get(activeChatMessages);
  const hasUserMessages = currentMessages.some((msg) => msg.role === "user");
  const currentChatData = get(currentChat);

  if (
    !hasUserMessages &&
    currentChatData &&
    (currentChatData.title === "New Chat" ||
      currentChatData.title.includes("New Chat"))
  ) {
    try {
      const newTitle = generateChatTitle(content);
      await updateChat(currentChatId, { title: newTitle });
    } catch (err) {
      console.warn("Failed to update parent chat title:", err);
      // Don't block parallel message sending if title update fails
    }
  }

  try {
    // Create branch chats for each model using the API client
    const response = await fetch(`/api/chats/${currentChatId}/parallel`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${localStorage.getItem("neko-auth-token")}`,
      },
      body: JSON.stringify({
        content,
        models,
      }),
    });

    if (!response.ok) {
      throw new Error("Failed to create parallel branches");
    }

    const branchChats = await response.json();

    // Update chats store with new branches
    chats.update((chatList) => [...branchChats, ...chatList]);

    // Rebuild tree structure
    const currentChats = await new Promise((resolve) => {
      chats.subscribe((chatList) => resolve(chatList))();
    });
    buildChatTree(currentChats);

    // Open right sidebar to show the graph
    rightSidebarCollapsed.set(false);

    // Switch to first branch immediately
    if (branchChats.length > 0) {
      await setActiveChat(branchChats[0].id);
    }

    // Start streaming for each branch
    branchChats.forEach((branchChat) => {
      streamingChats.update((set) => new Set(set).add(branchChat.id));
      startBranchStreaming(branchChat.id, content);
    });

    return branchChats;
  } catch (err) {
    console.error("Failed to send parallel message:", err);
    showError("Failed to send parallel message");
    throw err;
  }
}

// Start streaming for a branch chat
async function startBranchStreaming(chatId, content) {
  try {
    // Add user message to the branch chat first
    const userMessage = {
      id: `user-${Date.now()}-${chatId}`,
      role: "user",
      content,
      created_at: new Date().toISOString(),
    };

    // Add assistant message placeholder for streaming
    const assistantMessageId = `assistant-${Date.now()}-${chatId}`;
    const assistantMessage = {
      id: assistantMessageId,
      role: "assistant",
      content: "",
      created_at: new Date().toISOString(),
      streaming: true,
    };

    // Store the streaming message globally so we can access it when switching chats
    streamingMessages.update((messages) => ({
      ...messages,
      [chatId]: {
        userMessage,
        assistantMessage,
        assistantMessageId,
        content: "",
      },
    }));

    // If this is the active chat, update the UI immediately
    const currentActive = get(activeChat);
    if (currentActive === chatId) {
      addMessageToActiveChat(userMessage);
      addMessageToActiveChat(assistantMessage);
    }

    await chatAPI.streamMessage(chatId, content, {
      webSearch: false, // Parallel messages use individual model capabilities
      onChunk: (chunk, accumulatedContent) => {
        // Update the global streaming state
        streamingMessages.update((messages) => ({
          ...messages,
          [chatId]: {
            ...messages[chatId],
            content: accumulatedContent,
          },
        }));

        // Update messages if this chat is active
        const currentActiveChat = get(activeChat);
        if (currentActiveChat === chatId) {
          activeChatMessages.update((messages) =>
            messages.map((msg) =>
              msg.id === assistantMessageId
                ? { ...msg, content: accumulatedContent, streaming: true }
                : msg,
            ),
          );
        }
      },
      onComplete: async () => {
        // Mark streaming as complete globally
        streamingMessages.update((messages) => {
          const updated = { ...messages };
          if (updated[chatId]) {
            updated[chatId].assistantMessage.streaming = false;
          }
          return updated;
        });

        // Mark streaming as complete in UI if this is the active chat
        if (get(activeChat) === chatId) {
          activeChatMessages.update((messages) =>
            messages.map((msg) =>
              msg.id === assistantMessageId
                ? { ...msg, streaming: false }
                : msg,
            ),
          );

          // Reload messages from database to get the real IDs
          try {
            const messages = await chatAPI.getMessages(chatId);
            activeChatMessages.set(messages);
          } catch (err) {
            console.warn(
              "Failed to reload messages after parallel streaming:",
              err,
            );
          }
        }

        streamingChats.update((set) => {
          const newSet = new Set(set);
          newSet.delete(chatId);
          return newSet;
        });

        // Clean up the streaming message after completion
        setTimeout(() => {
          streamingMessages.update((messages) => {
            const updated = { ...messages };
            delete updated[chatId];
            return updated;
          });
        }, 1000);
      },
      onError: (error) => {
        console.error(`Streaming error for chat ${chatId}:`, error);

        // Update error state globally
        streamingMessages.update((messages) => {
          const updated = { ...messages };
          if (updated[chatId]) {
            updated[chatId].assistantMessage.content =
              "Error: Failed to get response";
            updated[chatId].assistantMessage.streaming = false;
            updated[chatId].assistantMessage.error = true;
          }
          return updated;
        });

        // Update error state if this is the active chat
        if (get(activeChat) === chatId) {
          activeChatMessages.update((messages) =>
            messages.map((msg) =>
              msg.id === assistantMessageId
                ? {
                    ...msg,
                    content: "Error: Failed to get response",
                    streaming: false,
                    error: true,
                  }
                : msg,
            ),
          );
        }

        streamingChats.update((set) => {
          const newSet = new Set(set);
          newSet.delete(chatId);
          return newSet;
        });

        // Clean up the streaming message after error
        setTimeout(() => {
          streamingMessages.update((messages) => {
            const updated = { ...messages };
            delete updated[chatId];
            return updated;
          });
        }, 1000);
      },
    });
  } catch (err) {
    console.error(`Failed to start streaming for chat ${chatId}:`, err);
    streamingChats.update((set) => {
      const newSet = new Set(set);
      newSet.delete(chatId);
      return newSet;
    });
  }
}

// Switch to a different branch
export async function switchToBranch(chatId) {
  // Simply navigate to the chat URL and let SvelteKit's routing handle the rest
  if (browser) {
    goto(`/chat/${chatId}`);
  }
}

// Track if chats have been initialized to prevent re-initialization
let chatsInitialized = false;

// Function to reset the initialization flag, e.g., on logout
export function resetChatsInitialized() {
  chatsInitialized = false;
}

// Initialize chats store
export function initializeChats() {
  if (!browser || chatsInitialized) return;

  // Only initialize once per session
  chatsInitialized = true;
  
  // Reset stores to initial state
  chats.set([]);
  activeChat.set(null);
  activeChatMessages.set([]);
  isLoading.set(false);
  error.set(null);
  chatTree.set({});
  streamingChats.set(new Set());
  streamingMessages.set({});
}
