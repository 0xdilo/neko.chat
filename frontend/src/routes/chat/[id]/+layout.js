import { browser } from '$app/environment';
import { setActiveChat, activeChat } from '$lib/stores/chats.js';
import { get } from 'svelte/store';

export const load = async ({ params, url }) => {
  const chatId = params.id;
  
  // Only set active chat on the client side to avoid SSR issues
  if (browser && chatId) {
    // Check if we need to change the active chat
    const currentActiveChat = get(activeChat);
    if (currentActiveChat !== chatId) {
      // Set the active chat for URL-based navigation
      // Make this blocking to ensure it completes before loadChats() runs
      try {
        await setActiveChat(chatId, false, false);
      } catch (error) {
        console.warn('Failed to load chat from URL:', error);
      }
    }
  }
  
  return {
    chatId,
    url: url.pathname
  };
};