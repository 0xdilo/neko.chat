import { api, endpoints, withErrorHandling } from './client.js';

// Chat API service
export const chatAPI = {
  // Get all chats for the current user
  async getChats(params = {}) {
    return withErrorHandling(
      () => api.get(endpoints.chats.list, params),
      'Failed to load chats.'
    );
  },

  // Create a new chat
  async createChat(chatData) {
    return withErrorHandling(
      () => api.post(endpoints.chats.create, {
        title: chatData.title || 'New Chat',
        system_prompt: chatData.system_prompt,
        provider: chatData.provider,
        model: chatData.model,
        is_branch: chatData.is_branch || false,
        parent_chat_id: chatData.parent_chat_id,
        branch_point_message_id: chatData.branch_point_message_id
      }),
      'Failed to create chat.'
    );
  },

  // Get a specific chat by ID
  async getChat(chatId) {
    return withErrorHandling(
      () => api.get(endpoints.chats.get(chatId)),
      'Failed to load chat.'
    );
  },

  // Update chat metadata
  async updateChat(chatId, updates) {
    return withErrorHandling(
      () => api.patch(endpoints.chats.update(chatId), updates),
      'Failed to update chat.'
    );
  },

  // Delete a chat
  async deleteChat(chatId) {
    return withErrorHandling(
      () => api.delete(endpoints.chats.delete(chatId)),
      'Failed to delete chat.'
    );
  },

  // Get messages for a chat
  async getMessages(chatId, params = {}) {
    return withErrorHandling(
      () => api.get(endpoints.chats.messages(chatId), params),
      'Failed to load messages.'
    );
  },

  // Send a message to a chat
  async sendMessage(chatId, message, options = {}) {
    return withErrorHandling(
      () => api.post(endpoints.chats.messages(chatId), {
        content: message
      }),
      'Failed to send message.'
    );
  },

  // Stream a message to a chat
  async streamMessage(chatId, message, options = {}) {
    try {
      const requestBody = {
        content: message
      };
      
      if (options.webSearch !== undefined) {
        requestBody.web_search = options.webSearch;
      }
      
      const response = await fetch(`/api/chats/${chatId}/stream`, {
        method: 'POST',
        headers: api.getHeaders(),
        body: JSON.stringify(requestBody)
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error('Response body is not readable');
      }

      const decoder = new TextDecoder();
      let accumulatedContent = '';

      try {
        while (true) {
          const { done, value } = await reader.read();
          
          if (done) {
            break;
          }

          const chunk = decoder.decode(value, { stream: true });
          accumulatedContent += chunk;
          
          // Call the onChunk callback with only the new chunk
          if (options.onChunk && chunk) {
            options.onChunk(chunk, accumulatedContent);
          }
        }

        // Call completion callback
        if (options.onComplete) {
          options.onComplete(accumulatedContent);
        }

        return accumulatedContent;
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      console.error('Streaming error:', error);
      if (options.onError) {
        options.onError(error);
      }
      throw error;
    }
  },

  // Regenerate response (for message editing - doesn't create new user message)
  async regenerateResponse(chatId, options = {}) {
    try {
      const response = await fetch(`/api/chats/${chatId}/regenerate`, {
        method: 'POST',
        headers: api.getHeaders()
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error('Response body is not readable');
      }

      const decoder = new TextDecoder();
      let accumulatedContent = '';

      try {
        while (true) {
          const { done, value } = await reader.read();
          
          if (done) {
            break;
          }

          const chunk = decoder.decode(value, { stream: true });
          accumulatedContent += chunk;
          
          // Call the onChunk callback with only the new chunk
          if (options.onChunk && chunk) {
            options.onChunk(chunk, accumulatedContent);
          }
        }

        // Call completion callback
        if (options.onComplete) {
          options.onComplete(accumulatedContent);
        }

        return accumulatedContent;
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      console.error('Regenerate streaming error:', error);
      if (options.onError) {
        options.onError(error);
      }
      throw error;
    }
  },

  // Update a message
  async updateMessage(chatId, messageId, updates) {
    return withErrorHandling(
      () => api.patch(`${endpoints.chats.messages(chatId)}/${messageId}`, updates),
      'Failed to update message.'
    );
  },

  // Delete a message
  async deleteMessage(chatId, messageId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.chats.messages(chatId)}/${messageId}`),
      'Failed to delete message.'
    );
  },

  // Delete subsequent messages after a given message (for editing)
  async deleteSubsequentMessages(chatId, messageId) {
    return withErrorHandling(
      () => api.delete(`${endpoints.chats.messages(chatId)}/${messageId}/subsequent`),
      'Failed to delete subsequent messages.'
    );
  },

  // Search chats and messages
  async searchChats(query, filters = {}) {
    return withErrorHandling(
      () => api.get('/api/chats/search', { 
        q: query,
        ...filters 
      }),
      'Search failed.'
    );
  },

  // Get chat statistics
  async getChatStats(chatId) {
    return withErrorHandling(
      () => api.get(`${endpoints.chats.get(chatId)}/stats`),
      'Failed to load chat statistics.'
    );
  },

  // Export chat data
  async exportChat(chatId, format = 'json') {
    return withErrorHandling(
      () => api.get(`${endpoints.chats.get(chatId)}/export`, { format }),
      'Failed to export chat.'
    );
  },

  // Import chat data
  async importChat(chatData) {
    return withErrorHandling(
      () => api.post('/api/chats/import', chatData),
      'Failed to import chat.'
    );
  },

  // Archive/unarchive chat
  async archiveChat(chatId, archived = true) {
    return withErrorHandling(
      () => api.patch(endpoints.chats.update(chatId), { archived }),
      `Failed to ${archived ? 'archive' : 'unarchive'} chat.`
    );
  },

  // Pin/unpin chat
  async pinChat(chatId, pinned = true) {
    return withErrorHandling(
      () => api.patch(endpoints.chats.update(chatId), { pinned }),
      `Failed to ${pinned ? 'pin' : 'unpin'} chat.`
    );
  },

  // Share chat (get shareable link)
  async shareChat(chatId, options = {}) {
    return withErrorHandling(
      () => api.post(`${endpoints.chats.get(chatId)}/share`, {
        expiresIn: options.expiresIn || '7d',
        password: options.password,
        allowComments: options.allowComments || false
      }),
      'Failed to create share link.'
    );
  },

  // Get shared chat (public access)
  async getSharedChat(shareToken) {
    return withErrorHandling(
      () => api.get(`/api/shared/chats/${shareToken}`),
      'Failed to load shared chat.'
    );
  },

  // Bulk insert messages (for branching functionality)
  async bulkInsertMessages(chatId, messages) {
    return withErrorHandling(
      () => api.post(`/api/chats/${chatId}/messages/bulk`, {
        messages: messages.map(msg => ({
          role: msg.role,
          content: msg.content
        }))
      }),
      'Failed to insert messages.'
    );
  },
};