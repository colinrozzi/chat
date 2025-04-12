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

// Also add to window object for global access
window.messageChain = messageChain;
window.currentHead = currentHead;
window.currentChatId = currentChatId;
window.chats = chats;
window.models = models;
window.ws = null; // Will be set later
window.reconnectAttempts = reconnectAttempts;
window.totalCost = totalCost;
window.lastUsedModelId = lastUsedModelId;
window.isWaitingForResponse = isWaitingForResponse;

export const MAX_RECONNECT_ATTEMPTS = 5;
export const RECONNECT_DELAY = 1000;

// Track token usage stats
export let totalInputTokens = 0;
export let totalOutputTokens = 0;
export let totalMessages = 0;

// Setter functions for mutable state
export function setMessageChain(newMessageChain) {
  messageChain = newMessageChain;
  window.messageChain = newMessageChain;
}

export function setCurrentHead(newCurrentHead) {
  currentHead = newCurrentHead;
  window.currentHead = newCurrentHead;
}

export function setCurrentChatId(newCurrentChatId) {
  currentChatId = newCurrentChatId;
  window.currentChatId = newCurrentChatId;
}

export function setChats(newChats) {
  chats = newChats;
  window.chats = newChats;
}

export function setModels(newModels) {
  models = newModels;
  window.models = newModels;
}

export function setWs(newWs) {
  ws = newWs;
  window.ws = newWs;
}

export function setReconnectAttempts(newReconnectAttempts) {
  reconnectAttempts = newReconnectAttempts;
  window.reconnectAttempts = newReconnectAttempts;
}

export function setTotalCost(newTotalCost) {
  totalCost = newTotalCost;
  window.totalCost = newTotalCost;
}

export function setLastUsedModelId(newLastUsedModelId) {
  lastUsedModelId = newLastUsedModelId;
  window.lastUsedModelId = newLastUsedModelId;
}

export function setIsWaitingForResponse(newIsWaitingForResponse) {
  isWaitingForResponse = newIsWaitingForResponse;
  window.isWaitingForResponse = newIsWaitingForResponse;
}

export function setTotalInputTokens(newTotalInputTokens) {
  totalInputTokens = newTotalInputTokens;
}

export function setTotalOutputTokens(newTotalOutputTokens) {
  totalOutputTokens = newTotalOutputTokens;
}

export function setTotalMessages(newTotalMessages) {
  totalMessages = newTotalMessages;
}

// Reset the state (useful for chat switching)
export function resetState() {
  messageChain = [];
  currentHead = null;
  lastUsedModelId = null;
  isWaitingForResponse = false;
  
  // Also reset window variables
  window.messageChain = [];
  window.currentHead = null;
  window.lastUsedModelId = null;
  window.isWaitingForResponse = false;
}

// Initialize the application
export function initializeApp() {
  console.log('Initializing chat application...');
  
  // Initialize UI components
  initializeSidebars();
  
  // Connect to WebSocket
  const wsConnection = connectWebSocket();
  setWs(wsConnection);
  
  // Check if global variables are properly set
  console.log('Global state initialization:', { 
    windowMessageChain: window.messageChain ? 'defined' : 'undefined',
    windowCurrentHead: window.currentHead !== undefined ? 'defined' : 'undefined',
    windowChats: window.chats ? 'defined' : 'undefined',
    windowWS: window.ws ? 'defined' : 'undefined',
  });
  
  console.log('Application initialization complete');
}
