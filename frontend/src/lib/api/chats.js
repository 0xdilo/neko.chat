import { api, endpoints, withErrorHandling } from "./client.js";

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
};
