import { browser } from "$app/environment";
import { writable } from "svelte/store";

export const wsConnected = writable(false);
export const wsConnecting = writable(false);
export const wsError = writable(null);

export const WS_MESSAGE_TYPES = {
  // Chat messages
  CHAT_MESSAGE: "chat_message",
  CHAT_UPDATE: "chat_update",
  CHAT_DELETE: "chat_delete",

  // Typing indicators
  TYPING_START: "typing_start",
  TYPING_STOP: "typing_stop",

  // User presence
  USER_ONLINE: "user_online",
  USER_OFFLINE: "user_offline",

  // System notifications
  SYSTEM_NOTIFICATION: "system_notification",

  // Real-time updates
  SETTINGS_UPDATE: "settings_update",
  USAGE_UPDATE: "usage_update",

  // Connection management
  PING: "ping",
  PONG: "pong",
  AUTH: "auth",
  AUTH_SUCCESS: "auth_success",
  AUTH_ERROR: "auth_error",

  // Errors
  ERROR: "error",
};

class WebSocketClient {
  constructor() {
    this.ws = null;
    this.url = null;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.reconnectDelay = 1000;
    this.heartbeatInterval = null;
    this.heartbeatTimeout = null;
    this.messageHandlers = new Map();
    this.connected = false;
    this.connecting = false;
  }

  // Connect to WebSocket server
  connect(url = null) {
    if (!browser) return;

    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      console.warn("WebSocket already connected");
      return;
    }

    this.url = url || this.getWebSocketURL();
    this.connecting = true;
    wsConnecting.set(true);
    wsError.set(null);

    try {
      this.ws = new WebSocket(this.url);
      this.setupEventHandlers();
    } catch (error) {
      console.error("Failed to create WebSocket connection:", error);
      this.handleError(error);
    }
  }

  // Get WebSocket URL
  getWebSocketURL() {
    if (!browser) return "";

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host.replace(":5173", ":8080"); // Development
    const token = localStorage.getItem("neko-auth-token");

    return `${protocol}//${host}/ws${token ? `?token=${token}` : ""}`;
  }

  // Setup WebSocket event handlers
  setupEventHandlers() {
    if (!this.ws) return;

    this.ws.onopen = () => {
      this.connected = true;
      this.connecting = false;
      this.reconnectAttempts = 0;

      wsConnected.set(true);
      wsConnecting.set(false);
      wsError.set(null);

      this.authenticate();
      this.startHeartbeat();
    };

    this.ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error("Failed to parse WebSocket message:", error);
      }
    };

    this.ws.onclose = (event) => {
      this.connected = false;
      this.connecting = false;

      wsConnected.set(false);
      wsConnecting.set(false);

      this.stopHeartbeat();

      if (
        !event.wasClean &&
        this.reconnectAttempts < this.maxReconnectAttempts
      ) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (error) => {
      console.error("WebSocket error:", error);
      this.handleError(error);
    };
  }

  // Handle incoming messages
  handleMessage(message) {
    const { type, data } = message;

    // Handle built-in message types
    switch (type) {
      case WS_MESSAGE_TYPES.PING:
        this.send({ type: WS_MESSAGE_TYPES.PONG });
        break;

      case WS_MESSAGE_TYPES.PONG:
        this.handlePong();
        break;

      case WS_MESSAGE_TYPES.AUTH_SUCCESS:
        break;

      case WS_MESSAGE_TYPES.AUTH_ERROR:
        console.error("WebSocket authentication failed:", data);
        this.disconnect();
        break;

      case WS_MESSAGE_TYPES.ERROR:
        console.error("WebSocket server error:", data);
        wsError.set(data.message || "Unknown server error");
        break;
    }

    // Call registered handlers
    const handlers = this.messageHandlers.get(type) || [];
    handlers.forEach((handler) => {
      try {
        handler(data, message);
      } catch (error) {
        console.error("Message handler error:", error);
      }
    });
  }

  // Send message to server
  send(message) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn("WebSocket not connected, cannot send message");
      return false;
    }

    try {
      this.ws.send(JSON.stringify(message));
      return true;
    } catch (error) {
      console.error("Failed to send WebSocket message:", error);
      return false;
    }
  }

  // Authenticate with server
  authenticate() {
    if (!browser) return;

    const token = localStorage.getItem("neko-auth-token");
    if (token) {
      this.send({
        type: WS_MESSAGE_TYPES.AUTH,
        data: { token },
      });
    }
  }

  // Start heartbeat to keep connection alive
  startHeartbeat() {
    this.heartbeatInterval = setInterval(() => {
      if (this.connected) {
        this.send({ type: WS_MESSAGE_TYPES.PING });

        // Set timeout for pong response
        this.heartbeatTimeout = setTimeout(() => {
          console.warn("Heartbeat timeout, reconnecting...");
          this.disconnect();
          this.connect();
        }, 5000);
      }
    }, 30000); // Send ping every 30 seconds
  }

  // Stop heartbeat
  stopHeartbeat() {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }

    if (this.heartbeatTimeout) {
      clearTimeout(this.heartbeatTimeout);
      this.heartbeatTimeout = null;
    }
  }

  // Handle pong response
  handlePong() {
    if (this.heartbeatTimeout) {
      clearTimeout(this.heartbeatTimeout);
      this.heartbeatTimeout = null;
    }
  }

  // Schedule reconnection attempt
  scheduleReconnect() {
    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    setTimeout(() => {
      if (!this.connected) {
        this.connect();
      }
    }, delay);
  }

  // Handle connection errors
  handleError(error) {
    this.connecting = false;
    wsConnecting.set(false);
    wsError.set(error.message || "WebSocket connection error");
  }

  // Disconnect from server
  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.connected = false;
    this.connecting = false;

    wsConnected.set(false);
    wsConnecting.set(false);

    this.stopHeartbeat();
  }

  // Register message handler
  on(messageType, handler) {
    if (!this.messageHandlers.has(messageType)) {
      this.messageHandlers.set(messageType, []);
    }

    this.messageHandlers.get(messageType).push(handler);

    // Return unsubscribe function
    return () => {
      const handlers = this.messageHandlers.get(messageType) || [];
      const index = handlers.indexOf(handler);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    };
  }

  // Remove message handler
  off(messageType, handler) {
    const handlers = this.messageHandlers.get(messageType) || [];
    const index = handlers.indexOf(handler);
    if (index > -1) {
      handlers.splice(index, 1);
    }
  }

  // Remove all handlers for message type
  removeAllHandlers(messageType) {
    this.messageHandlers.delete(messageType);
  }

  // Get connection status
  isConnected() {
    return this.connected;
  }

  // Get connecting status
  isConnecting() {
    return this.connecting;
  }
}

// Create global WebSocket client instance
export const wsClient = new WebSocketClient();

// Convenience functions for common operations
export const websocket = {
  // Connect to WebSocket
  connect: (url) => wsClient.connect(url),

  // Disconnect from WebSocket
  disconnect: () => wsClient.disconnect(),

  // Send message
  send: (message) => wsClient.send(message),

  // Register event handler
  on: (type, handler) => wsClient.on(type, handler),

  // Remove event handler
  off: (type, handler) => wsClient.off(type, handler),

  // Send chat message
  sendChatMessage: (chatId, content) => {
    return wsClient.send({
      type: WS_MESSAGE_TYPES.CHAT_MESSAGE,
      data: { chatId, content },
    });
  },

  // Send typing indicator
  sendTypingStart: (chatId) => {
    return wsClient.send({
      type: WS_MESSAGE_TYPES.TYPING_START,
      data: { chatId },
    });
  },

  sendTypingStop: (chatId) => {
    return wsClient.send({
      type: WS_MESSAGE_TYPES.TYPING_STOP,
      data: { chatId },
    });
  },

  // Connection status
  isConnected: () => wsClient.isConnected(),
  isConnecting: () => wsClient.isConnecting(),
};

// Auto-connect when browser is available
if (browser && typeof window !== "undefined") {
  // Connect when authentication is available
  const token = localStorage.getItem("neko-auth-token");
  if (token) {
    websocket.connect();
  }
}
