import { writable, derived, get } from "svelte/store";
import { browser } from "$app/environment";
import { goto } from "$app/navigation";
import { chatAPI } from "$lib/api/chats.js";
import { websocket, WS_MESSAGE_TYPES } from "$lib/api/websocket.js";
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

// Utility function to clean up streaming state for a chat
function cleanupStreamingState(chatId) {
  streamingChats.update((set) => {
    const newSet = new Set(set);
    newSet.delete(chatId);
    return newSet;
  });

  streamingMessages.update((messages) => {
    const updated = { ...messages };
    delete updated[chatId];
    return updated;
  });
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
        newTree[chat.parentChatId].children = newTree[
          chat.parentChatId
        ].children.filter((child) => child.id !== chat.id);
      }
    } else {
      // Add or update chat in tree
      const chatWithChildren = {
        ...chat,
        children: currentTree[chat.id]?.children || [],
      };

      if (chat.parentChatId && newTree[chat.parentChatId]) {
        // It's a branch - add to parent's children
        const parentChildren = newTree[chat.parentChatId].children || [];
        const existingIndex = parentChildren.findIndex(
          (child) => child.id === chat.id,
        );
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
    const modelToUse =
      chatData.provider && chatData.model
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

export async function setActiveChat(
  chatId,
  preserveMessages = false,
  updateUrl = true,
) {
  if (!chatId) {
    activeChat.set(null);
    activeChatMessages.set([]);
    if (updateUrl && browser) {
      goto("/");
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
    
    // Check if there's an ongoing stream for this chat
    const allStreamingMessages = get(streamingMessages);
    const streamingData = allStreamingMessages[chatId];
    
    if (streamingData) {
      const { userMessage, assistantMessage, content } = streamingData;
      let messagesToDisplay = [...messages];

      console.log(`setActiveChat: Found streaming data for chat ${chatId}, content length: ${content.length}`);

      // Ensure user message is displayed
      if (userMessage && !messages.some(m => m.role === 'user')) {
        messagesToDisplay.unshift(userMessage);
      }

      // Rehydrate the view with the streaming assistant message
      const existingAssistantMessage = messagesToDisplay.find(m => m.id === assistantMessage.id);
      if (!existingAssistantMessage) {
        const messageToDisplay = {
          ...assistantMessage,
          content: content, // Use the accumulated content from streaming state
          streaming: true,
        };
        messagesToDisplay.push(messageToDisplay);
        console.log(`setActiveChat: Added new streaming message with ${content.length} chars`);
      } else {
        // IMPORTANT: Always use the content from streaming state, never from database
        // The database content might be stale/empty while streaming
        messagesToDisplay = messagesToDisplay.map(m => 
          m.id === assistantMessage.id ? { ...m, content: content, streaming: true } : m
        );
        console.log(`setActiveChat: Updated existing message with ${content.length} chars`);
      }
      activeChatMessages.set(messagesToDisplay);
    } else {
      console.log(`setActiveChat: No streaming data found for chat ${chatId}`);
      activeChatMessages.set(messages);
    }
  } catch (err) {
    console.error("Failed to load messages:", err);
    showError("Failed to load messages");
    activeChatMessages.set([]);
  }
}
("");

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
    const deletedChat = currentChats.find((chat) => chat.id === chatId);

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
          goto("/");
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
    // Clear any stale streaming state first  
    cleanupStreamingState(currentChatId);
    
    // Stream message response using WebSocket streaming
    const streamMethod = chatAPI.streamMessage;
    await streamMethod(currentChatId, content, {
      webSearch: options.webSearch,
      onStart: options.onStart,
      onChunk: (chunk, accumulatedContent) => {
        console.log(`Store onChunk called: chunk="${chunk.substring(0, 50)}...", accumulated=${accumulatedContent.length} chars`);
        
        // Update global streaming state immediately
        streamingMessages.update((messages) => ({
          ...messages,
          [currentChatId]: {
            ...messages[currentChatId],
            content: accumulatedContent,
          },
        }));

        // Throttle UI updates - still use accumulated content for backward compatibility
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
        cleanupStreamingState(currentChatId);

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
        cleanupStreamingState(currentChatId);

        showError("Failed to get response");
      },
      ...options,
    });

    return { success: true };
  } catch (err) {
    console.error("Failed to send message:", err);

    // If stream was aborted by user, just mark as completed without error
    if (err.name === "AbortError") {
      // The message should already have the partial content from onChunk calls
      // Just mark it as not streaming anymore
      updateMessageInActiveChat(assistantMessageId, {
        streaming: false,
      });

      // Clean up global streaming state
      cleanupStreamingState(currentChatId);

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

  // Deduplicate models to prevent creating multiple branches for the same provider/model
  const uniqueModels = [];
  const seen = new Set();
  for (const model of models) {
    const key = `${model.provider}:${model.model}`;
    if (!seen.has(key)) {
      seen.add(key);
      uniqueModels.push(model);
    }
  }

  // Create a temporary user message object to pass to streaming functions.
  // This helps bridge the gap until the backend-persisted message is available.
  const userMessage = {
    id: `user-${Date.now()}`,
    role: "user",
    content,
    created_at: new Date().toISOString(),
  };

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
    // Check if current chat uses one of the selected models
    let chatsToStream = [];
    let currentChatModel = null;
    
    if (currentChatData) {
      currentChatModel = uniqueModels.find(m => 
        m.provider === currentChatData.provider && 
        m.model === currentChatData.model
      );
    }
    
    // Create branches for models NOT used by the current chat
    const modelsNeedingBranches = uniqueModels.filter(m => 
      !currentChatModel || 
      (m.provider !== currentChatModel.provider || m.model !== currentChatModel.model)
    );
    
    // Open right sidebar immediately to show the graph
    rightSidebarCollapsed.set(false);
    
    // If current chat uses one of the selected models, start streaming in it
    if (currentChatModel) {
      chatsToStream.push({
        id: currentChatId,
        provider: currentChatModel.provider,
        model: currentChatModel.model,
        isCurrentChat: true
      });
      
      // Start streaming in current chat (non-blocking)
      sendMessage(content, {
        provider: currentChatModel.provider,
        model: currentChatModel.model,
        webSearch: options.webSearch || false,
        onStart: (controller) => {
          if (options.onStart) {
            options.onStart(controller);
          }
        }
      }).catch(err => console.error("Error streaming in current chat:", err));
    }
    
    // Handle branches - create them while finding the fork point
    if (modelsNeedingBranches.length > 0) {
      // Start a process to find user message and create branches
      (async () => {
        try {
          let userMessageForForking = null;
          let attempts = 0;
          const maxAttempts = 10;
          
          // If no current model, we need to create the user message first
          if (!currentChatModel) {
            await sendMessage(content, {
              webSearch: false,
              onStart: (controller) => {
                setTimeout(() => controller.abort(), 50);
              }
            });
          }
          
          // Get messages to find the new user message for forking
          await new Promise(resolve => setTimeout(resolve, 300)); // Give time for messages to be saved
          const messages = await chatAPI.getMessages(currentChatId);
          
          // Find the NEW user message (C) that we just added
          let newUserMessage = null;
          for (let i = messages.length - 1; i >= 0; i--) {
            if (messages[i].role === 'user' && messages[i].content === content) {
              newUserMessage = messages[i];
              break;
            }
          }
          
          if (!newUserMessage) {
            console.error("Failed to find new user message for branching");
            return;
          }
          
          console.log(`Forking from new user message: ${newUserMessage.content.substring(0, 50)}... - ID: ${newUserMessage.id}`);
          
          // Create all branches in parallel
          const branchPromises = modelsNeedingBranches.map(async (model) => {
            try {
              const branchChat = await chatAPI.createFork(currentChatId, newUserMessage.id, {
                provider: model.provider,
                model: model.model,
                send_message: false  // Don't re-send, just fork from this point
              });
              
              // Update UI immediately
              chats.update((chatList) => [branchChat, ...chatList]);
              
              // Start streaming immediately - the backend will handle the new user message
              streamingChats.update((set) => new Set(set).add(branchChat.id));
              // Use regenerate since the backend already added the user message
              startBranchStreaming(branchChat.id, content);
              
              return branchChat;
            } catch (error) {
              console.error(`Failed to create branch for model ${model.model}:`, error);
              return null;
            }
          });
          
          // Wait for all branches
          const branches = await Promise.all(branchPromises);
          
          // Rebuild tree structure
          const currentChats = get(chats);
          buildChatTree(currentChats);
          
        } catch (err) {
          console.error("Error in branch creation:", err);
        }
      })();
    }

    return chatsToStream;
  } catch (err) {
    console.error("Failed to send parallel message:", err);
    showError("Failed to send parallel message");
    throw err;
  }
}

// Start streaming for a branch chat
async function startBranchStreaming(chatId, content) {
  try {
    // The backend has already created the user message.
    // We just manage the assistant's streaming response.

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
        userMessage: null, // Branch already has user message from backend
        assistantMessage,
        assistantMessageId,
        content: "",
      },
    }));

    // If this is the active chat, update the UI immediately
    const currentActive = get(activeChat);
    if (currentActive === chatId) {
      // Don't add userMessage here - the branch already has it from the backend
      // Only add the assistant message placeholder
      addMessageToActiveChat(assistantMessage);
    }

    const streamMethod = chatAPI.regenerateResponse;
    await streamMethod(chatId, {
      webSearch: false, // Parallel messages use individual model capabilities
      onChunk: (chunk, accumulatedContent) => {
        // Update the global streaming state
        streamingMessages.update((messages) => {
          const updatedMessages = { ...messages };
          if (updatedMessages[chatId]) {
            updatedMessages[chatId].content = accumulatedContent;
          }
          return updatedMessages;
        });

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
        // Update the global state with the final content
        streamingMessages.update((messages) => {
          const updated = { ...messages };
          if (updated[chatId]) {
            updated[chatId].content = get(activeChatMessages).find(m => m.id === assistantMessageId)?.content || "";
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
        }

        // Clean up streaming state after completion
        cleanupStreamingState(chatId);

        // Clean up the streaming message after completion (with timeout for UI transition)
        setTimeout(() => {
          cleanupStreamingState(chatId);
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

        // Clean up streaming state after error
        cleanupStreamingState(chatId);

        // Clean up the streaming message after error (with timeout for UI transition)
        setTimeout(() => {
          cleanupStreamingState(chatId);
        }, 1000);
      },
    });
  } catch (err) {
    console.error(`Failed to start streaming for chat ${chatId}:`, err);
    cleanupStreamingState(chatId);
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

  // Set up global streaming message handlers when WebSocket is connected
  setupGlobalStreamingHandlers();
  
  // Wait for WebSocket connection before requesting stream resume
  const checkConnectionAndResume = () => {
    const token = localStorage.getItem("neko-auth-token");
    if (token && websocket && websocket.isConnected()) {
      // Request stream resume for any active streams
      websocket.send({
        type: "RequestStreamResume",
        data: {}
      });
    } else if (token) {
      // If we have a token but no connection, wait a bit more
      setTimeout(checkConnectionAndResume, 100);
    }
  };
  
  // Initial check after a small delay
  setTimeout(checkConnectionAndResume, 200);
}

// Setup global handlers for streaming messages
function setupGlobalStreamingHandlers() {
  // Handle streaming resume messages
  websocket.on(WS_MESSAGE_TYPES.STREAMING_RESUME, (data) => {
    const chatId = data.chat_id;
    const content = data.content || "";
    const messageId = data.message_id;
    
    console.log(`Resuming stream for chat ${chatId} with ${content.length} characters:`, content.substring(0, 100) + '...');
    
    // Small delay to ensure page load is complete before resume
    setTimeout(() => {
    
    // Add to global streaming state
    streamingChats.update((set) => new Set(set).add(chatId));
    
    // Create or update streaming message data with accumulated content
    streamingMessages.update((messages) => ({
      ...messages,
      [chatId]: {
        userMessage: null, // Will be filled when we get messages
        assistantMessage: { id: messageId, role: "assistant", content, streaming: true, created_at: new Date().toISOString() },
        assistantMessageId: messageId,
        content: content,
      },
    }));
    
    // Force update the UI immediately, regardless of active chat
    // This ensures the content is displayed before setActiveChat potentially overwrites it
    const currentActiveChat = get(activeChat);
    if (currentActiveChat === chatId) {
      console.log(`Updating active chat UI with resumed content: ${content.length} chars`);
      
      // Force immediate update of the active chat messages
      activeChatMessages.update((messages) => {
        console.log(`Current messages count: ${messages.length}`);
        const existingIndex = messages.findIndex(m => m.id === messageId);
        
        if (existingIndex >= 0) {
          // Update existing message with full content
          const updatedMessages = [...messages];
          updatedMessages[existingIndex] = { 
            ...updatedMessages[existingIndex], 
            content, 
            streaming: true 
          };
          console.log(`Updated existing message at index ${existingIndex} with ${content.length} chars`);
          return updatedMessages;
        } else {
          // Add new assistant message with full accumulated content
          const newMessage = { 
            id: messageId, 
            role: "assistant", 
            content, 
            streaming: true, 
            created_at: new Date().toISOString() 
          };
          console.log(`Added new streaming message with ${content.length} chars`);
          return [...messages, newMessage];
        }
      });
    }
    }, 100); // 100ms delay to ensure page load is complete
  });
  
  // Handle streaming updates
  websocket.on(WS_MESSAGE_TYPES.STREAMING_UPDATE, (data) => {
    const chatId = data.chat_id;
    const contentDelta = data.content_delta || "";
    
    // Update global streaming state - use full content from backend instead of delta
    streamingMessages.update((messages) => {
      if (messages[chatId]) {
        // Use the full content sent by backend instead of accumulating deltas
        messages[chatId].content = data.content || (messages[chatId].content + contentDelta);
      }
      return { ...messages };
    });
    
    // If this is the active chat, update the UI
    const currentActiveChat = get(activeChat);
    if (currentActiveChat === chatId) {
      // Use the full content directly from the backend
      const fullContent = data.content || "";
      
      console.log(`Updating UI for streaming message ${data.message_id}: ${fullContent.length} chars`);
      
      // Update the message with the full content from backend
      activeChatMessages.update((messages) =>
        messages.map((msg) =>
          msg.id === data.message_id
            ? { ...msg, content: fullContent, streaming: true }
            : msg,
        ),
      );
    }
  });
  
  // Handle streaming complete
  websocket.on(WS_MESSAGE_TYPES.STREAMING_COMPLETE, (data) => {
    const chatId = data.chat_id;
    const messageId = data.message_id;
    const content = data.content; // May contain final content for completed streams
    
    console.log(`Stream completed for chat ${chatId}, final content: ${content ? content.length : 0} chars`);
    
    const currentActiveChat = get(activeChat);
    if (currentActiveChat === chatId) {
      // Always update the message to mark streaming as complete
      activeChatMessages.update((messages) => {
        const existingIndex = messages.findIndex(m => m.id === messageId);
        if (existingIndex >= 0) {
          // Update existing message - use final content if provided, otherwise keep existing
          const updatedMessages = [...messages];
          const existingMessage = updatedMessages[existingIndex];
          updatedMessages[existingIndex] = { 
            ...existingMessage, 
            content: content && content.length > 0 ? content : existingMessage.content,
            streaming: false 
          };
          console.log(`Updated message ${messageId} with final content: ${updatedMessages[existingIndex].content.length} chars`);
          return updatedMessages;
        } else {
          // Add new message with final content (for reconnect cases)
          if (content && content.length > 0) {
            return [...messages, { 
              id: messageId, 
              role: "assistant", 
              content, 
              streaming: false, 
              created_at: new Date().toISOString() 
            }];
          }
          return messages;
        }
      });
    }
    
    // Remove from streaming state
    cleanupStreamingState(chatId);
    
    // If this is the active chat and no content was provided, just mark as complete
    if (!content || content.length === 0) {
      const currentActiveChat = get(activeChat);
      if (currentActiveChat === chatId) {
        updateMessageInActiveChat(messageId, { streaming: false });
      }
    }
  });
  
  // Handle streaming errors
  websocket.on(WS_MESSAGE_TYPES.STREAMING_ERROR, (data) => {
    const chatId = data.chat_id;
    const messageId = data.message_id;
    const error = data.error || "Streaming error occurred";
    
    console.error(`Stream error for chat ${chatId}: ${error}`);
    
    // Remove from streaming state
    cleanupStreamingState(chatId);
    
    // If this is the active chat, mark message as error
    const currentActiveChat = get(activeChat);
    if (currentActiveChat === chatId) {
      updateMessageInActiveChat(messageId, { 
        content: `Error: ${error}`,
        streaming: false,
        error: true 
      });
    }
  });
}
