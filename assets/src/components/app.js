// Core application initialization
import { connectWebSocket } from '../services/websocket.js';
import { elements } from '../utils/elements.js';
import { initializeSidebars } from '../utils/ui.js';

// Global state - exported for use in other modules
export let messageChain = [];
export let currentHead = null;
export let currentChatId = null;
export let chats = [];
export let models = [];
export let ws = null;
export let reconnectAttempts = 0;
export let totalCost = 0;
export let lastUsedModelId = null; // Track the last used model ID
export let isWaitingForResponse = false;

export const MAX_RECONNECT_ATTEMPTS = 5;
export const RECONNECT_DELAY = 1000;

// Track token usage stats
export let totalInputTokens = 0;
export let totalOutputTokens = 0;
export let totalMessages = 0;

// Reset the state (useful for chat switching)
export function resetState() {
  messageChain = [];
  currentHead = null;
  lastUsedModelId = null;
  isWaitingForResponse = false;
}

// Initialize the application
export function initializeApp() {
  console.log('Initializing chat application...');
  
  // Initialize UI components
  initializeSidebars();
  
  // Connect to WebSocket
  connectWebSocket();
  
  console.log('Application initialization complete');
}
