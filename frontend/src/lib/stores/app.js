import { writable, derived } from 'svelte/store';

// App-wide state
export const isOnline = writable(true);
export const isLoading = writable(false);
export const loadingMessage = writable('');
export const sidebarCollapsed = writable(false);
export const currentRoute = writable('/');

// Notifications system
export const notifications = writable([]);

// Modal system
export const activeModal = writable(null);
export const modalData = writable(null);

// Command palette
export const showCommandPalette = writable(false);
export const commandPaletteQuery = writable('');

// Search
export const searchQuery = writable('');
export const searchResults = writable([]);
export const isSearching = writable(false);

// Create notification
export function createNotification(notification) {
  const id = Date.now();
  const newNotification = {
    id,
    type: 'info', // info, success, warning, error
    title: '',
    message: '',
    duration: 5000,
    dismissible: true,
    actions: [],
    ...notification,
    createdAt: new Date()
  };
  
  notifications.update(current => [...current, newNotification]);
  
  // Auto-dismiss if duration is set
  if (newNotification.duration > 0) {
    setTimeout(() => {
      dismissNotification(id);
    }, newNotification.duration);
  }
  
  return id;
}

// Dismiss notification
export function dismissNotification(id) {
  notifications.update(current => current.filter(n => n.id !== id));
}

// Clear all notifications
export function clearNotifications() {
  notifications.set([]);
}

// Show success notification
export function showSuccess(message, title = 'Success') {
  return createNotification({
    type: 'success',
    title,
    message,
    duration: 3000
  });
}

// Show error notification
export function showError(message, title = 'Error') {
  return createNotification({
    type: 'error',
    title,
    message,
    duration: 7000
  });
}

// Show warning notification
export function showWarning(message, title = 'Warning') {
  return createNotification({
    type: 'warning',
    title,
    message,
    duration: 5000
  });
}

// Show info notification
export function showInfo(message, title = 'Info') {
  return createNotification({
    type: 'info',
    title,
    message,
    duration: 4000
  });
}

// Modal management
export function openModal(modalType, data = null) {
  activeModal.set(modalType);
  modalData.set(data);
}

export function closeModal() {
  activeModal.set(null);
  modalData.set(null);
}

// Loading states
export function setLoading(loading, message = '') {
  isLoading.set(loading);
  loadingMessage.set(message);
}

export function showLoading(message = 'Loading...') {
  setLoading(true, message);
}

export function hideLoading() {
  setLoading(false, '');
}

// Sidebar management
export function toggleSidebar() {
  sidebarCollapsed.update(collapsed => !collapsed);
}

export function setSidebarCollapsed(collapsed) {
  sidebarCollapsed.set(collapsed);
}

// Command palette
export function openCommandPalette() {
  showCommandPalette.set(true);
  commandPaletteQuery.set('');
}

export function closeCommandPalette() {
  showCommandPalette.set(false);
  commandPaletteQuery.set('');
}

// Search functionality
export function setSearchQuery(query) {
  searchQuery.set(query);
}

export function setSearchResults(results) {
  searchResults.set(results);
}

export function setSearching(searching) {
  isSearching.set(searching);
}

export async function performSearch(query) {
  if (!query.trim()) {
    setSearchResults([]);
    return;
  }
  
  setSearching(true);
  
  try {
    // Simulate search API call
    await new Promise(resolve => setTimeout(resolve, 300));
    
    // Mock search results
    const mockResults = [
      {
        id: 1,
        type: 'chat',
        title: 'General Discussion',
        content: 'How can I help you today?',
        matches: 1
      },
      {
        id: 2,
        type: 'message',
        title: 'In Code Review',
        content: 'The implementation looks good...',
        matches: 1
      }
    ].filter(item => 
      item.title.toLowerCase().includes(query.toLowerCase()) ||
      item.content.toLowerCase().includes(query.toLowerCase())
    );
    
    setSearchResults(mockResults);
  } catch (error) {
    console.error('Search failed:', error);
    setSearchResults([]);
    showError('Search failed. Please try again.');
  } finally {
    setSearching(false);
  }
}

// Route management
export function setCurrentRoute(route) {
  currentRoute.set(route);
}

// Network status
export function setOnlineStatus(online) {
  isOnline.set(online);
  
  if (online) {
    showSuccess('Back online', 'Connection restored');
  } else {
    showWarning('You are offline', 'Connection lost');
  }
}

// Initialize network status monitoring
export function initializeNetworkMonitoring() {
  if (typeof window === 'undefined') return;
  
  // Set initial status
  setOnlineStatus(navigator.onLine);
  
  // Listen for network changes
  window.addEventListener('online', () => setOnlineStatus(true));
  window.addEventListener('offline', () => setOnlineStatus(false));
}

// Derived stores
export const hasNotifications = derived(
  notifications,
  $notifications => $notifications.length > 0
);

export const unreadNotifications = derived(
  notifications,
  $notifications => $notifications.filter(n => !n.read)
);

export const isModalOpen = derived(
  activeModal,
  $activeModal => $activeModal !== null
);

export const hasSearchResults = derived(
  searchResults,
  $searchResults => $searchResults.length > 0
);

// App initialization
export function initializeApp() {
  initializeNetworkMonitoring();
  
  // Set initial route if in browser
  if (typeof window !== 'undefined') {
    setCurrentRoute(window.location.pathname);
  }
}

// Cleanup function
export function cleanup() {
  if (typeof window === 'undefined') return;
  
  window.removeEventListener('online', () => setOnlineStatus(true));
  window.removeEventListener('offline', () => setOnlineStatus(false));
}

// Keyboard shortcuts integration
export function registerGlobalShortcuts() {
  if (typeof window === 'undefined') return;
  
  window.addEventListener('keydown', (event) => {
    // Command palette: Ctrl/Cmd + K
    if ((event.ctrlKey || event.metaKey) && event.key === 'k') {
      event.preventDefault();
      openCommandPalette();
    }
    
    // Escape: Close modals/overlays
    if (event.key === 'Escape') {
      if (activeModal.subscribe(modal => modal)()) {
        closeModal();
      } else if (showCommandPalette.subscribe(show => show)()) {
        closeCommandPalette();
      }
    }
  });
}

// Error boundary
export function handleGlobalError(error, errorInfo) {
  console.error('Global error:', error, errorInfo);
  
  showError(
    'An unexpected error occurred. Please refresh the page.',
    'Application Error'
  );
}

// Performance monitoring
export const performanceMetrics = writable({
  pageLoadTime: 0,
  apiResponseTimes: [],
  renderTimes: []
});

export function recordMetric(type, value) {
  performanceMetrics.update(metrics => ({
    ...metrics,
    [type]: type === 'pageLoadTime' ? value : [...metrics[type], value]
  }));
}