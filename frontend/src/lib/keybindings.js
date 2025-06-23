import { writable } from "svelte/store";
import { browser } from "$app/environment";

// Store for active keybinding mode
export const keybindingMode = writable("normal");

// Store for keybinding sequences
export const commandSequence = writable("");

// Default keybindings configuration
export const defaultKeybindings = {
  // Navigation
  j: { action: "scroll_down", description: "Scroll down" },
  k: { action: "scroll_up", description: "Scroll up" },
  gg: { action: "scroll_top", description: "Go to top" },
  G: { action: "scroll_bottom", description: "Go to bottom" },
  h: { action: "toggle_sidebar", description: "Toggle sidebar" },
  l: { action: "focus_main", description: "Focus main content" },

  // Chat actions
  o: { action: "new_chat", description: "New chat" },
  i: { action: "focus_input", description: "Focus message input" },
  "/": { action: "search", description: "Search chats" },

  // Modal/UI actions
  Escape: { action: "escape", description: "Close modal/cancel" },
  Enter: { action: "submit", description: "Submit/confirm" },

  // Application shortcuts
  "ctrl+n": { action: "new_chat", description: "New chat" },
  "ctrl+o": { action: "new_chat", description: "New chat" },
  "ctrl+k": { action: "command_palette", description: "Command palette" },
  "ctrl+m": { action: "model_modal", description: "Open model modal" },
  "ctrl+b": { action: "toggle_sidebar", description: "Toggle sidebar" },
  "ctrl+,": { action: "settings", description: "Open settings" },
  "ctrl+t": { action: "toggle_theme", description: "Toggle theme" },

  // Chat navigation
  "ctrl+j": { action: "next_chat", description: "Next chat" },
  "ctrl+k": { action: "prev_chat", description: "Previous chat" },

  // Numbers for chat selection
  1: { action: "select_chat_1", description: "Select chat 1" },
  2: { action: "select_chat_2", description: "Select chat 2" },
  3: { action: "select_chat_3", description: "Select chat 3" },
  4: { action: "select_chat_4", description: "Select chat 4" },
  5: { action: "select_chat_5", description: "Select chat 5" },
  6: { action: "select_chat_6", description: "Select chat 6" },
  7: { action: "select_chat_7", description: "Select chat 7" },
  8: { action: "select_chat_8", description: "Select chat 8" },
  9: { action: "select_chat_9", description: "Select chat 9" },
};

// Global event handlers store
export const eventHandlers = writable({});

// Scroll utilities
export function smoothScrollTo(element, target) {
  if (!element) return;

  element.scrollTo({
    top: target,
    behavior: "smooth",
  });
}

export function scrollByAmount(element, amount) {
  if (!element) return;

  const current = element.scrollTop;
  smoothScrollTo(element, current + amount);
}

// Key sequence handling
let sequenceBuffer = "";
let sequenceTimeout = null;

export function createKeybindingSystem() {
  if (!browser)
    return {
      registerHandler: () => {},
      unregisterHandler: () => {},
      enable: () => {},
      disable: () => {},
      setMode: () => {},
      executeAction: () => {},
      destroy: () => {},
    };

  let handlers = {};
  let isEnabled = true;
  let mode = "normal";

  eventHandlers.set(handlers);

  function isInputFocused() {
    const activeElement = document.activeElement;
    return (
      activeElement &&
      (activeElement.tagName === "INPUT" ||
        activeElement.tagName === "TEXTAREA" ||
        activeElement.contentEditable === "true")
    );
  }

  function getScrollableParent(element) {
    if (!element) return document.documentElement;

    const parent = element.parentElement;
    if (!parent) return document.documentElement;

    const computedStyle = window.getComputedStyle(parent);
    if (
      computedStyle.overflowY === "auto" ||
      computedStyle.overflowY === "scroll"
    ) {
      return parent;
    }

    return getScrollableParent(parent);
  }

  function executeAction(action, event) {
    const mainElement =
      document.querySelector(".messages-container") ||
      document.querySelector(".settings-main") ||
      document.querySelector("main");

    const sidebarElement = document.querySelector(".sidebar");
    const inputElement =
      document.querySelector(".message-input") ||
      document.querySelector('input[type="text"]');

    switch (action) {
      case "scroll_down":
        if (mainElement) {
          scrollByAmount(mainElement, 100);
        }
        break;

      case "scroll_up":
        if (mainElement) {
          scrollByAmount(mainElement, -100);
        }
        break;

      case "scroll_top":
        if (mainElement) {
          smoothScrollTo(mainElement, 0);
        }
        break;

      case "scroll_bottom":
        if (mainElement) {
          smoothScrollTo(mainElement, mainElement.scrollHeight);
        }
        break;

      case "toggle_sidebar":
        // Dispatch custom event for sidebar toggle
        window.dispatchEvent(new CustomEvent("toggle-sidebar"));
        break;

      case "focus_main":
        if (mainElement) {
          mainElement.focus();
        }
        break;

      case "new_chat":
        window.dispatchEvent(new CustomEvent("new-chat"));
        break;

      case "focus_input":
        if (inputElement) {
          inputElement.focus();
        }
        break;

      case "search":
        window.dispatchEvent(new CustomEvent("search-chats"));
        break;

      case "escape":
        // Remove focus from any input
        if (document.activeElement) {
          document.activeElement.blur();
        }
        // Close modals
        window.dispatchEvent(new CustomEvent("escape-action"));
        break;

      case "command_palette":
        window.dispatchEvent(new CustomEvent("command-palette"));
        break;

      case "model_modal":
        window.dispatchEvent(new CustomEvent("model-modal"));
        break;

      case "settings":
        window.location.href = "/settings";
        break;

      case "toggle_theme":
        window.dispatchEvent(new CustomEvent("toggle-theme"));
        break;

      case "next_chat":
        window.dispatchEvent(new CustomEvent("next-chat"));
        break;

      case "prev_chat":
        window.dispatchEvent(new CustomEvent("prev-chat"));
        break;

      default:
        if (action.startsWith("select_chat_")) {
          const chatNumber = action.split("_")[2];
          window.dispatchEvent(
            new CustomEvent("select-chat", {
              detail: { chatNumber: parseInt(chatNumber) },
            }),
          );
        }
        break;
    }
  }

  function handleKeyPress(event) {
    if (!isEnabled) return;

    // Don't intercept when typing in inputs (except for specific keys)
    if (isInputFocused() && !["Escape", "Enter"].includes(event.key)) {
      return;
    }

    const key = event.key;
    const ctrl = event.ctrlKey;
    const alt = event.altKey;
    const shift = event.shiftKey;

    let keyCombo = "";
    if (ctrl) keyCombo += "ctrl+";
    if (alt) keyCombo += "alt+";
    if (shift) keyCombo += "shift+";
    keyCombo += key.toLowerCase();

    if (!ctrl && !alt && key.length === 1 && !isInputFocused()) {
      clearTimeout(sequenceTimeout);
      sequenceBuffer += key;

      const binding = defaultKeybindings[sequenceBuffer];
      if (binding) {
        event.preventDefault();
        executeAction(binding.action, event);
        sequenceBuffer = "";
        commandSequence.set("");
        return;
      }

      const hasPartialMatch = Object.keys(defaultKeybindings).some(
        (k) => k.startsWith(sequenceBuffer) && k.length > sequenceBuffer.length,
      );

      if (hasPartialMatch) {
        commandSequence.set(sequenceBuffer);
        sequenceTimeout = setTimeout(() => {
          sequenceBuffer = "";
          commandSequence.set("");
        }, 1000);
      } else {
        sequenceBuffer = "";
        commandSequence.set("");
      }
    }

    const binding = defaultKeybindings[keyCombo] || defaultKeybindings[key];
    if (binding && (!isInputFocused() || ["Escape", "Enter"].includes(key))) {
      event.preventDefault();
      executeAction(binding.action, event);
    }
  }

  function registerHandler(name, handler) {
    handlers[name] = handler;
    eventHandlers.set(handlers);
  }

  function unregisterHandler(name) {
    delete handlers[name];
    eventHandlers.set(handlers);
  }

  function enable() {
    isEnabled = true;
  }

  function disable() {
    isEnabled = false;
  }

  function setMode(newMode) {
    mode = newMode;
    keybindingMode.set(mode);
  }

  document.addEventListener("keydown", handleKeyPress);

  return {
    registerHandler,
    unregisterHandler,
    enable,
    disable,
    setMode,
    executeAction,
    destroy: () => {
      document.removeEventListener("keydown", handleKeyPress);
      clearTimeout(sequenceTimeout);
    },
  };
}
