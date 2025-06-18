import { writable, derived, get } from "svelte/store";
import { browser } from "$app/environment";
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

// Load chats from API
export async function loadChats() {
  isLoading.set(true);
  error.set(null);

  try {
    const chatList = await chatAPI.getChats();
    chats.set(chatList);
    buildChatTree(chatList);

    // Set the most recent chat as active if none is set
    if (chatList.length > 0) {
      const mostRecent = chatList[0]; // Already sorted by backend
      await setActiveChat(mostRecent.id);
    }
  } catch (err) {
    console.error("Failed to load chats:", err);
    error.set(err.message);
    showError("Failed to load chats");
  } finally {
    isLoading.set(false);
  }
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
    });

    chats.update((chatList) => [newChat, ...chatList]);
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
export async function setActiveChat(chatId, preserveMessages = false) {
  if (!chatId) {
    activeChat.set(null);
    activeChatMessages.set([]);
    return;
  }

  if (!preserveMessages) {
    activeChatMessages.set([]);
  }
  activeChat.set(chatId);

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

    // Update chats store and rebuild tree
    chats.update((chatList) => {
      const updatedChatList = chatList.filter((chat) => chat.id !== chatId);
      // Rebuild tree with updated chat list
      buildChatTree(updatedChatList);
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

  try {
    // Stream message response
    await chatAPI.streamMessage(currentChatId, content, {
      webSearch: options.webSearch,
      onChunk: (accumulatedContent) => {
        // Update global streaming state
        streamingMessages.update((messages) => ({
          ...messages,
          [currentChatId]: {
            ...messages[currentChatId],
            content: accumulatedContent,
          },
        }));

        // Update the assistant message with accumulated streaming content
        updateMessageInActiveChat(assistantMessageId, {
          content: accumulatedContent,
          streaming: true,
        });
      },
      onComplete: async () => {
        // Mark streaming as complete
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

        // Reload messages from database only if this is the active chat
        try {
          const messages = await chatAPI.getMessages(currentChatId);
          // Only update activeChatMessages if this is still the active chat
          if (get(activeChat) === currentChatId) {
            activeChatMessages.set(messages);
          }
        } catch (err) {
          console.warn("Failed to reload messages after streaming:", err);
        }
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
    // Update the assistant message with error
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
export async function sendParallelMessage(content, models) {
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
      onChunk: (accumulatedContent) => {
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
  const isStreaming = get(streamingChats).has(chatId);
  const streamingData = get(streamingMessages)[chatId];

  if (isStreaming && streamingData) {
    // For streaming chats, preserve current messages and manually manage the switch
    activeChat.set(chatId);

    // Check if we have messages loaded
    const messages = get(activeChatMessages);

    // Check if the streaming messages are already in the UI
    const hasUserMessage = messages.some(
      (msg) => msg.id === streamingData.userMessage.id,
    );
    const hasStreamingMessage = messages.some(
      (msg) => msg.id === streamingData.assistantMessageId,
    );

    // Load messages from database first
    try {
      const dbMessages = await chatAPI.getMessages(chatId);

      // Check if streaming messages are already in database
      const userInDb = dbMessages.some(
        (msg) =>
          msg.role === "user" &&
          msg.content === streamingData.userMessage.content,
      );
      const assistantInDb = dbMessages.some(
        (msg) =>
          msg.role === "assistant" &&
          msg.id === streamingData.assistantMessageId,
      );

      let currentMessages = [...dbMessages];

      // Only add streaming messages if they're not already in the database
      if (!userInDb) {
        currentMessages.push(streamingData.userMessage);
      }

      if (!assistantInDb) {
        const assistantMessage = {
          ...streamingData.assistantMessage,
          content: streamingData.content || "",
          streaming: true,
        };
        currentMessages.push(assistantMessage);
      } else {
        // Update existing message in database with streaming content
        currentMessages = currentMessages.map((msg) =>
          msg.role === "assistant" &&
          msg.id !== streamingData.assistantMessageId
            ? msg
            : msg.role === "assistant"
              ? {
                  ...msg,
                  content: streamingData.content || "",
                  streaming: true,
                }
              : msg,
        );
      }

      activeChatMessages.set(currentMessages);
    } catch (err) {
      console.error("Failed to load messages for streaming chat:", err);
      // Fallback to just showing streaming messages
      activeChatMessages.set([
        streamingData.userMessage,
        {
          ...streamingData.assistantMessage,
          content: streamingData.content || "",
          streaming: true,
        },
      ]);
    }

    // Set up a reactive subscription to update UI when streaming content changes
    const unsubscribe = streamingMessages.subscribe((messages) => {
      const currentStreamingData = messages[chatId];
      if (currentStreamingData && get(activeChat) === chatId) {
        updateMessageInActiveChat(streamingData.assistantMessageId, {
          content: currentStreamingData.content || "",
          streaming: true,
        });
      }
    });

    // Clean up subscription when streaming completes or chat changes
    const originalStreamingChats = get(streamingChats);
    if (originalStreamingChats.has(chatId)) {
      const checkComplete = setInterval(() => {
        const currentStreamingChats = get(streamingChats);
        const currentActiveChat = get(activeChat);
        if (
          !currentStreamingChats.has(chatId) ||
          currentActiveChat !== chatId
        ) {
          unsubscribe();
          clearInterval(checkComplete);
        }
      }, 100);
    }
  } else {
    // For non-streaming chats, use normal setActiveChat
    await setActiveChat(chatId);
  }
}

// Initialize chats store
export function initializeChats() {
  if (!browser) return;

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
