import { api, endpoints, withErrorHandling } from "./client.js";
import { websocket, WS_MESSAGE_TYPES } from "./websocket.js";

// Helper function to extract error message from response
async function extractErrorMessage(response) {
  const errorText = await response.text();
  let errorMessage = `HTTP ${response.status}`;

  try {
    const errorData = JSON.parse(errorText);
    errorMessage = errorData.message || errorData.error || errorMessage;
  } catch (e) {
    if (errorText) errorMessage = errorText;
  }

  return errorMessage;
}

// Helper function to cleanup WebSocket event handlers
function cleanupWebSocketHandlers(handlers) {
  handlers.forEach(({ type, handler }) => {
    websocket.off(type, handler);
  });
}

// Helper function to create streaming handlers
function createStreamingHandlers(chatId, abortController, accumulatedContentRef, options, resolve, reject, isCompletedRef, streamIdRef, messageIdRef) {
  const handlers = {
    update: (data) => {
      if (data.chat_id === chatId && !abortController.signal.aborted) {
        // Always use the full content from backend if available, as it's the authoritative source
        if (data.content !== undefined && data.content !== null) {
          accumulatedContentRef.value = data.content;
        } else if (data.content_delta) {
          // Only accumulate deltas if no full content is provided
          accumulatedContentRef.value += data.content_delta;
        }
        
        if (options.onChunk) {
          // For backward compatibility, still send the delta and accumulated content
          options.onChunk(data.content_delta || "", accumulatedContentRef.value);
        }
      }
    },

    start: (data) => {
      if (data.chat_id === chatId) {
        streamIdRef.value = data.stream_id;
        messageIdRef.value = data.message_id;
      }
    },

    resume: (data) => {
      if (data.chat_id === chatId && !abortController.signal.aborted) {
        // Resume from where we left off - use the full content from the backend
        if (data.content !== undefined && data.content !== null) {
          accumulatedContentRef.value = data.content;
        }
        
        console.log(`Enhanced stream resume: ${accumulatedContentRef.value.length} characters for chat ${chatId}`);
        
        if (options.onChunk && accumulatedContentRef.value) {
          // Call onChunk with the full accumulated content to restore the UI state
          options.onChunk(accumulatedContentRef.value, accumulatedContentRef.value);
        }
      }
    },

    complete: (data) => {
      if (data.chat_id === chatId && !isCompletedRef.value) {
        isCompletedRef.value = true;
        cleanupWebSocketHandlers(handlerList);
        
        if (options.onComplete) {
          options.onComplete(accumulatedContentRef.value);
        }
        resolve(accumulatedContentRef.value);
      }
    },

    error: (data) => {
      if (data.chat_id === chatId && !isCompletedRef.value) {
        isCompletedRef.value = true;
        cleanupWebSocketHandlers(handlerList);
        
        const error = new Error(data.error || 'Streaming error occurred');
        if (options.onError) {
          options.onError(error);
        }
        reject(error);
      }
    }
  };

  const handlerList = [
    { type: WS_MESSAGE_TYPES.STREAMING_UPDATE, handler: handlers.update },
    { type: WS_MESSAGE_TYPES.STREAMING_START, handler: handlers.start },
    { type: WS_MESSAGE_TYPES.STREAMING_RESUME, handler: handlers.resume },
    { type: WS_MESSAGE_TYPES.STREAMING_COMPLETE, handler: handlers.complete },
    { type: WS_MESSAGE_TYPES.STREAMING_ERROR, handler: handlers.error }
  ];

  return handlerList;
}

export const chatAPI = {
  async getChats(params = {}) {
    return withErrorHandling(
      () => api.get(endpoints.chats.list, params),
      "Failed to load chats.",
    );
  },

  async createChat(chatData) {
    return withErrorHandling(
      () =>
        api.post(endpoints.chats.create, {
          title: chatData.title || "New Chat",
          system_prompt: chatData.system_prompt,
          provider: chatData.provider,
          model: chatData.model,
          is_branch: chatData.is_branch || false,
          parent_chat_id: chatData.parent_chat_id,
          branch_point_message_id: chatData.branch_point_message_id,
        }),
      "Failed to create chat.",
    );
  },

  async getChat(chatId) {
    return withErrorHandling(
      () => api.get(endpoints.chats.get(chatId)),
      "Failed to load chat.",
    );
  },

  async updateChat(chatId, updates) {
    return withErrorHandling(
      () => api.patch(endpoints.chats.update(chatId), updates),
      "Failed to update chat.",
    );
  },

  async deleteChat(chatId) {
    return withErrorHandling(
      () => api.delete(endpoints.chats.delete(chatId)),
      "Failed to delete chat.",
    );
  },

  async getMessages(chatId, params = {}) {
    return withErrorHandling(
      () => api.get(endpoints.chats.messages(chatId), params),
      "Failed to load messages.",
    );
  },

  async sendMessage(chatId, message, options = {}) {
    return withErrorHandling(
      () =>
        api.post(endpoints.chats.messages(chatId), {
          content: message,
        }),
      "Failed to send message.",
    );
  },


  async updateMessage(chatId, messageId, updates) {
    return withErrorHandling(
      () =>
        api.patch(`${endpoints.chats.messages(chatId)}/${messageId}`, updates),
      "Failed to update message.",
    );
  },

  async deleteMessage(chatId, messageId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.chats.messages(chatId)}/${messageId}`),
      "Failed to delete message.",
    );
  },

  async deleteSubsequentMessages(chatId, messageId) {
    return withErrorHandling(
      () =>
        api.delete(
          `${endpoints.chats.messages(chatId)}/${messageId}/subsequent`,
        ),
      "Failed to delete subsequent messages.",
    );
  },

  async bulkInsertMessages(chatId, messages) {
    return withErrorHandling(
      () =>
        api.post(`/api/chats/${chatId}/messages/bulk`, {
          messages: messages.map((msg) => ({
            role: msg.role,
            content: msg.content,
          })),
        }),
      "Failed to insert messages.",
    );
  },

  async createFork(chatId, messageId, options = {}) {
    return withErrorHandling(
      () =>
        api.post(`/api/chats/${chatId}/fork`, {
          message_id: messageId,
          provider: options.provider,
          model: options.model,
          send_message: options.send_message,
        }),
      "Failed to create fork.",
    );
  },

  async streamMessage(chatId, message, options = {}) {
    console.log("Starting stream for chat:", chatId);

    return new Promise((resolve, reject) => {
      // Use object wrappers for mutable references
      const accumulatedContentRef = { value: "" };
      const streamIdRef = { value: null };
      const messageIdRef = { value: null };
      const isCompletedRef = { value: false };

      // Create abort controller for cancellation
      const abortController = new AbortController();
      if (options.onStart) {
        options.onStart(abortController);
      }

      // Handle abort signal
      abortController.signal.addEventListener('abort', () => {
        if (streamIdRef.value) {
          // Cancel stream on backend
          fetch(`/api/streaming/states/${streamIdRef.value}/cancel`, {
            method: 'DELETE',
            headers: api.getHeaders(),
          }).catch(console.error);
        }
        if (!isCompletedRef.value) {
          resolve(accumulatedContentRef.value);
        }
      });

      // Create and register WebSocket event handlers
      const handlerList = createStreamingHandlers(
        chatId, abortController, accumulatedContentRef, options, 
        resolve, reject, isCompletedRef, streamIdRef, messageIdRef
      );

      handlerList.forEach(({ type, handler }) => {
        websocket.on(type, handler);
      });

      // Initiate streaming via HTTP API
      const requestBody = {
        content: message,
      };

      if (options.webSearch !== undefined) {
        requestBody.web_search = options.webSearch;
      }

      console.log("Initiating stream request:", {
        chatId,
        requestBody,
        url: `/api/v2/chats/${chatId}/stream`
      });

      fetch(`/api/v2/chats/${chatId}/stream`, {
        method: "POST",
        headers: api.getHeaders(),
        body: JSON.stringify(requestBody),
        signal: abortController.signal,
      })
      .then(async (response) => {
        console.log("Stream response status:", response.status);
        if (!response.ok) {
          console.error("Stream request failed:", {
            status: response.status,
            statusText: response.statusText
          });
          const errorMessage = await extractErrorMessage(response);
          throw new Error(errorMessage);
        } else {
          console.log("Stream request successful");
        }
        // For WebSocket streaming, we don't need to process the HTTP response body
        // The actual streaming happens through WebSocket messages
      })
      .catch((error) => {
        if (!isCompletedRef.value && !abortController.signal.aborted) {
          isCompletedRef.value = true;
          cleanupWebSocketHandlers(handlerList);
          
          if (options.onError) {
            options.onError(error);
          }
          reject(error);
        }
      });
    });
  },

  async regenerateResponse(chatId, options = {}) {
    console.log("Starting regenerate for chat:", chatId);

    return new Promise((resolve, reject) => {
      // Use object wrappers for mutable references
      const accumulatedContentRef = { value: "" };
      const streamIdRef = { value: null };
      const messageIdRef = { value: null };
      const isCompletedRef = { value: false };

      // Create abort controller for cancellation
      const abortController = new AbortController();
      if (options.onStart) {
        options.onStart(abortController);
      }

      // Handle abort signal
      abortController.signal.addEventListener('abort', () => {
        if (streamIdRef.value) {
          // Cancel stream on backend
          fetch(`/api/streaming/states/${streamIdRef.value}/cancel`, {
            method: 'DELETE',
            headers: api.getHeaders(),
          }).catch(console.error);
        }
        if (!isCompletedRef.value) {
          resolve(accumulatedContentRef.value);
        }
      });

      // Create and register WebSocket event handlers
      const handlerList = createStreamingHandlers(
        chatId, abortController, accumulatedContentRef, options, 
        resolve, reject, isCompletedRef, streamIdRef, messageIdRef
      );

      handlerList.forEach(({ type, handler }) => {
        websocket.on(type, handler);
      });

      // Initiate regeneration via HTTP API
      fetch(`/api/v2/chats/${chatId}/regenerate`, {
        method: "POST",
        headers: api.getHeaders(),
        signal: abortController.signal,
      })
      .then(async (response) => {
        if (!response.ok) {
          const errorMessage = await extractErrorMessage(response);
          throw new Error(errorMessage);
        }
        // For WebSocket streaming, we don't need to process the HTTP response body
        // The actual streaming happens through WebSocket messages
      })
      .catch((error) => {
        if (!isCompletedRef.value && !abortController.signal.aborted) {
          isCompletedRef.value = true;
          cleanupWebSocketHandlers(handlerList);
          
          if (options.onError) {
            options.onError(error);
          }
          reject(error);
        }
      });
    });
  },
};
