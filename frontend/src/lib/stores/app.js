import { writable, derived } from "svelte/store";

// App-wide state
export const isLoading = writable(false);
export const sidebarCollapsed = writable(false);

// Notifications system
export const notifications = writable([]);

export function createNotification(notification) {
  const id = Date.now();
  const newNotification = {
    id,
    type: "info",
    title: "",
    message: "",
    duration: 5000,
    dismissible: true,
    actions: [],
    ...notification,
    createdAt: new Date(),
  };

  notifications.update((current) => [...current, newNotification]);

  if (newNotification.duration > 0) {
    setTimeout(() => {
      dismissNotification(id);
    }, newNotification.duration);
  }

  return id;
}

export function dismissNotification(id) {
  notifications.update((current) => current.filter((n) => n.id !== id));
}

export function showSuccess(message, title = "Success") {
  return createNotification({
    type: "success",
    title,
    message,
    duration: 3000,
  });
}

export function showError(message, title = "Error") {
  return createNotification({
    type: "error",
    title,
    message,
    duration: 7000,
  });
}

export function showWarning(message, title = "Warning") {
  return createNotification({
    type: "warning",
    title,
    message,
    duration: 5000,
  });
}

