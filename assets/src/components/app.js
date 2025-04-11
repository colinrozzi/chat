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

// Setter functions for mutable state
export function setMessageChain(newMessageChain) {
  messageChain = newMessageChain;
}

export function setCurrentHead(newCurrentHead) {
  currentHead = newCurrentHead;
}

export function setCurrentChatId(newCurrentChatId) {
  currentChatId = newCurrentChatId;
}

export function setChats(newChats) {
  chats = newChats;
}

export function setModels(newModels) {
  models = newModels;
}

export function setWs(newWs) {
  ws = newWs;
}

export function setReconnectAttempts(newReconnectAttempts) {
  reconnectAttempts = newReconnectAttempts;
}

export function setTotalCost(newTotalCost) {
  totalCost = newTotalCost;
}

export function setLastUsedModelId(newLastUsedModelId) {
  lastUsedModelId = newLastUsedModelId;
}

export function setIsWaitingForResponse(newIsWaitingForResponse) {
  isWaitingForResponse = newIsWaitingForResponse;
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
}

// Initialize the application
export function initializeApp() {
  console.log('Initializing chat application...');
  
  // Initialize UI components
  initializeSidebars();
  
  // Connect to WebSocket
  const wsConnection = connectWebSocket();
  setWs(wsConnection);
  
  console.log('Application initialization complete');
}
