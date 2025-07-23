import { api, endpoints, withErrorHandling } from "./client.js";
import { websocket, WS_MESSAGE_TYPES, USE_ENHANCED_STREAMING } from "./websocket.js";

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

  async streamMessage(chatId, message, options = {}) {
    let reader = null;
    let abortController = new AbortController();
    let accumulatedContent = "";

    if (options.onStart) {
      options.onStart(abortController);
    }

    try {
      const requestBody = {
        content: message,
      };

      if (options.webSearch !== undefined) {
        requestBody.web_search = options.webSearch;
      }

      const response = await fetch(`/api/chats/${chatId}/stream`, {
        method: "POST",
        headers: api.getHeaders(),
        body: JSON.stringify(requestBody),
        signal: abortController.signal,
      });

      if (!response.ok) {
        const errorText = await response.text();
        let errorMessage = `HTTP ${response.status}`;

        try {
          const errorData = JSON.parse(errorText);
          errorMessage = errorData.message || errorData.error || errorMessage;
        } catch (e) {
          if (errorText) errorMessage = errorText;
        }

        throw new Error(errorMessage);
      }

      reader = response.body?.getReader();
      if (!reader) {
        throw new Error("Response body is not readable");
      }

      const decoder = new TextDecoder();

      try {
        while (true) {
          const { done, value } = await reader.read();

          if (done) {
            break;
          }

          const chunk = decoder.decode(value, { stream: true });

          if (chunk.startsWith("ERROR:")) {
            const errorMsg = chunk.substring(6).trim();
            throw new Error(errorMsg || "Streaming error occurred");
          }

          accumulatedContent += chunk;

          if (options.onChunk && chunk) {
            options.onChunk(chunk, accumulatedContent);
          }
        }

        if (options.onComplete) {
          options.onComplete(accumulatedContent);
        }

        return accumulatedContent;
      } finally {
        if (reader) {
          reader.releaseLock();
        }
      }
    } catch (error) {
      console.error("Streaming error:", error);

      if (error.name === "AbortError") {
        return accumulatedContent || "";
      }

      if (options.onError) {
        options.onError(error);
      }
      throw error;
    }
  },

  async regenerateResponse(chatId, options = {}) {
    let reader = null;
    let abortController = new AbortController();
    let accumulatedContent = "";

    if (options.onStart) {
      options.onStart(abortController);
    }

    try {
      const response = await fetch(`/api/chats/${chatId}/regenerate`, {
        method: "POST",
        headers: api.getHeaders(),
        signal: abortController.signal,
      });

      if (!response.ok) {
        const errorText = await response.text();
        let errorMessage = `HTTP ${response.status}`;

        try {
          const errorData = JSON.parse(errorText);
          errorMessage = errorData.message || errorData.error || errorMessage;
        } catch (e) {
          if (errorText) errorMessage = errorText;
        }

        throw new Error(errorMessage);
      }

      reader = response.body?.getReader();
      if (!reader) {
        throw new Error("Response body is not readable");
      }

      const decoder = new TextDecoder();

      try {
        while (true) {
          const { done, value } = await reader.read();

          if (done) {
            break;
          }

          const chunk = decoder.decode(value, { stream: true });

          if (chunk.startsWith("ERROR:")) {
            const errorMsg = chunk.substring(6).trim();
            throw new Error(errorMsg || "Streaming error occurred");
          }

          accumulatedContent += chunk;

          if (options.onChunk && chunk) {
            options.onChunk(chunk, accumulatedContent);
          }
        }

        if (options.onComplete) {
          options.onComplete(accumulatedContent);
        }

        return accumulatedContent;
      } finally {
        if (reader) {
          reader.releaseLock();
        }
      }
    } catch (error) {
      console.error("Regenerate streaming error:", error);

      if (error.name === "AbortError") {
        return accumulatedContent || "";
      }

      if (options.onError) {
        options.onError(error);
      }
      throw error;
    }
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

  // Enhanced WebSocket-based streaming methods
  async enhancedStreamMessage(chatId, message, options = {}) {
    if (!USE_ENHANCED_STREAMING) {
      return this.streamMessage(chatId, message, options);
    }

    return new Promise((resolve, reject) => {
      let accumulatedContent = "";
      let streamId = null;
      let messageId = null;
      let isCompleted = false;

      // Create abort controller for cancellation
      const abortController = new AbortController();
      if (options.onStart) {
        options.onStart(abortController);
      }

      // Handle abort signal
      abortController.signal.addEventListener('abort', () => {
        if (streamId) {
          // Cancel stream on backend
          fetch(`/api/streaming/states/${streamId}/cancel`, {
            method: 'DELETE',
            headers: api.getHeaders(),
          }).catch(console.error);
        }
        if (!isCompleted) {
          resolve(accumulatedContent);
        }
      });

      // Register WebSocket handlers for this streaming session
      const handleStreamingUpdate = (data) => {
        if (data.chat_id === chatId && !abortController.signal.aborted) {
          // Use full content from backend instead of accumulating deltas
          accumulatedContent = data.content || accumulatedContent + (data.content_delta || "");
          
          if (options.onChunk) {
            // For backward compatibility, still send the delta and accumulated content
            options.onChunk(data.content_delta || "", accumulatedContent);
          }
        }
      };

      const handleStreamingStart = (data) => {
        if (data.chat_id === chatId) {
          streamId = data.stream_id;
          messageId = data.message_id;
        }
      };

      const handleStreamingResume = (data) => {
        if (data.chat_id === chatId && !abortController.signal.aborted) {
          // Resume from where we left off
          accumulatedContent = data.content || "";
          
          console.log(`Enhanced stream resume: ${accumulatedContent.length} characters for chat ${chatId}`);
          
          if (options.onChunk && accumulatedContent) {
            // Call onChunk with the full accumulated content to restore the UI state
            options.onChunk(accumulatedContent, accumulatedContent);
          }
        }
      };

      const handleStreamingComplete = (data) => {
        if (data.chat_id === chatId && !isCompleted) {
          isCompleted = true;
          websocket.off(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);
          
          if (options.onComplete) {
            options.onComplete(accumulatedContent);
          }
          resolve(accumulatedContent);
        }
      };

      const handleStreamingError = (data) => {
        if (data.chat_id === chatId && !isCompleted) {
          isCompleted = true;
          websocket.off(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);
          
          const error = new Error(data.error || 'Streaming error occurred');
          if (options.onError) {
            options.onError(error);
          }
          reject(error);
        }
      };

      // Register WebSocket event handlers
      websocket.on(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);

      // Initiate streaming via HTTP API
      const requestBody = {
        content: message,
      };

      if (options.webSearch !== undefined) {
        requestBody.web_search = options.webSearch;
      }

      fetch(`/api/v2/chats/${chatId}/stream`, {
        method: "POST",
        headers: api.getHeaders(),
        body: JSON.stringify(requestBody),
        signal: abortController.signal,
      })
      .then(async (response) => {
        if (!response.ok) {
          const errorText = await response.text();
          let errorMessage = `HTTP ${response.status}`;

          try {
            const errorData = JSON.parse(errorText);
            errorMessage = errorData.message || errorData.error || errorMessage;
          } catch (e) {
            if (errorText) errorMessage = errorText;
          }

          throw new Error(errorMessage);
        }
        // For WebSocket streaming, we don't need to process the HTTP response body
        // The actual streaming happens through WebSocket messages
      })
      .catch((error) => {
        if (!isCompleted && !abortController.signal.aborted) {
          isCompleted = true;
          websocket.off(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);
          
          if (options.onError) {
            options.onError(error);
          }
          reject(error);
        }
      });
    });
  },

  async enhancedRegenerateResponse(chatId, options = {}) {
    if (!USE_ENHANCED_STREAMING) {
      return this.regenerateResponse(chatId, options);
    }

    return new Promise((resolve, reject) => {
      let accumulatedContent = "";
      let streamId = null;
      let messageId = null;
      let isCompleted = false;

      // Create abort controller for cancellation
      const abortController = new AbortController();
      if (options.onStart) {
        options.onStart(abortController);
      }

      // Handle abort signal
      abortController.signal.addEventListener('abort', () => {
        if (streamId) {
          // Cancel stream on backend
          fetch(`/api/streaming/states/${streamId}/cancel`, {
            method: 'DELETE',
            headers: api.getHeaders(),
          }).catch(console.error);
        }
        if (!isCompleted) {
          resolve(accumulatedContent);
        }
      });

      // Register WebSocket handlers for this streaming session
      const handleStreamingUpdate = (data) => {
        if (data.chat_id === chatId && !abortController.signal.aborted) {
          // Use full content from backend instead of accumulating deltas
          accumulatedContent = data.content || accumulatedContent + (data.content_delta || "");
          
          if (options.onChunk) {
            // For backward compatibility, still send the delta and accumulated content
            options.onChunk(data.content_delta || "", accumulatedContent);
          }
        }
      };

      const handleStreamingStart = (data) => {
        if (data.chat_id === chatId) {
          streamId = data.stream_id;
          messageId = data.message_id;
        }
      };

      const handleStreamingResume = (data) => {
        if (data.chat_id === chatId && !abortController.signal.aborted) {
          // Resume from where we left off
          accumulatedContent = data.content || "";
          
          console.log(`Enhanced stream resume: ${accumulatedContent.length} characters for chat ${chatId}`);
          
          if (options.onChunk && accumulatedContent) {
            // Call onChunk with the full accumulated content to restore the UI state
            options.onChunk(accumulatedContent, accumulatedContent);
          }
        }
      };

      const handleStreamingComplete = (data) => {
        if (data.chat_id === chatId && !isCompleted) {
          isCompleted = true;
          websocket.off(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);
          
          if (options.onComplete) {
            options.onComplete(accumulatedContent);
          }
          resolve(accumulatedContent);
        }
      };

      const handleStreamingError = (data) => {
        if (data.chat_id === chatId && !isCompleted) {
          isCompleted = true;
          websocket.off(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);
          
          const error = new Error(data.error || 'Streaming error occurred');
          if (options.onError) {
            options.onError(error);
          }
          reject(error);
        }
      };

      // Register WebSocket event handlers
      websocket.on(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
      websocket.on(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);

      // Initiate regeneration via HTTP API
      fetch(`/api/v2/chats/${chatId}/regenerate`, {
        method: "POST",
        headers: api.getHeaders(),
        signal: abortController.signal,
      })
      .then(async (response) => {
        if (!response.ok) {
          const errorText = await response.text();
          let errorMessage = `HTTP ${response.status}`;

          try {
            const errorData = JSON.parse(errorText);
            errorMessage = errorData.message || errorData.error || errorMessage;
          } catch (e) {
            if (errorText) errorMessage = errorText;
          }

          throw new Error(errorMessage);
        }
        // For WebSocket streaming, we don't need to process the HTTP response body
        // The actual streaming happens through WebSocket messages
      })
      .catch((error) => {
        if (!isCompleted && !abortController.signal.aborted) {
          isCompleted = true;
          websocket.off(WS_MESSAGE_TYPES.STREAMING_UPDATE, handleStreamingUpdate);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_START, handleStreamingStart);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_RESUME, handleStreamingResume);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_COMPLETE, handleStreamingComplete);
          websocket.off(WS_MESSAGE_TYPES.STREAMING_ERROR, handleStreamingError);
          
          if (options.onError) {
            options.onError(error);
          }
          reject(error);
        }
      });
    });
  },
};
