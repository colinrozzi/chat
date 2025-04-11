// Event handling setup for the chat interface
import { elements } from '../utils/elements.js';
import { connectWebSocket } from '../services/websocket.js';
import { sendMessage, generateLlmResponse, createNewChat, branchChat } from './chat.js';
import { toggleChatSidebar, toggleChatControlsSidebar, toggleSection, scrollToBottom } from '../utils/ui.js';
import { copyMessageText, copyMessageId } from '../utils/clipboard.js';
import { updateModelInfo } from './model-selector.js';

// Setup all event listeners
export function setupEventListeners() {
  console.log('Setting up event listeners...');
  
  // Auto-resize textarea and update button states
  elements.messageInput?.addEventListener('input', () => {
    elements.messageInput.style.height = 'auto';
    elements.messageInput.style.height = Math.min(elements.messageInput.scrollHeight, 120) + 'px';
    elements.sendButton.disabled = !elements.messageInput.value.trim();
  });
  
  // Add keyboard shortcut for focusing message input
  document.addEventListener('keydown', (event) => {
    if (event.key === '\\') {
      event.preventDefault(); // Prevent the \ from being typed
      elements.messageInput?.focus();
    }
  });
  
  // One event handler for all keyboard shortcuts in the message input
  elements.messageInput?.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      // Shift+Enter to send message
      if (event.shiftKey) {
        event.preventDefault();
        console.log('Sending message with Shift+Enter');
        sendMessage(window.ws);
      }
      // Ctrl+Enter or Cmd+Enter to generate response
      else if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        console.log('Generating response with Ctrl/Cmd+Enter');
        generateLlmResponse(window.ws);
      }
    }
  });
  
  // Button click handlers
  elements.sendButton?.addEventListener('click', () => sendMessage(window.ws));
  elements.generateButton?.addEventListener('click', () => generateLlmResponse(window.ws));
  
  // Chat sidebar toggle handlers
  elements.collapseChatSidebarButton?.addEventListener('click', toggleChatSidebar);
  elements.expandChatSidebarButton?.addEventListener('click', toggleChatSidebar);
  
  // Chat controls sidebar toggle handlers
  elements.collapseChatControlsButton?.addEventListener('click', toggleChatControlsSidebar);
  elements.expandChatControlsButton?.addEventListener('click', toggleChatControlsSidebar);
  
  // Model selector change handler
  elements.controlsModelSelector?.addEventListener('change', updateModelInfo);
  
  // New chat button
  elements.newChatButton?.addEventListener('click', () => createNewChat(window.ws));
  
  // Branch chat button
  elements.branchChatButton?.addEventListener('click', () => branchChat(window.ws));
  
  // Expose certain functions to the global scope for use in inline event handlers
  // This is necessary because our bundled code won't have these functions directly accessible
  window.switchChat = (chatId) => import('./chat.js').then(m => m.switchChat(chatId, window.ws));
  window.showRenameChat = (chatId, name) => import('./chat.js').then(m => m.showRenameChat(chatId, name));
  window.confirmDeleteChat = (chatId) => import('./chat.js').then(m => m.confirmDeleteChat(chatId));
  window.copyMessageText = (messageId) => import('../utils/clipboard.js').then(m => m.copyMessageText(messageId));
  window.copyMessageId = (messageId) => import('../utils/clipboard.js').then(m => m.copyMessageId(messageId));
  window.toggleSection = (sectionId) => import('../utils/ui.js').then(m => m.toggleSection(sectionId));
  
  console.log('Event listeners setup complete');
}

// Cleanup function to remove event listeners
export function cleanupEventListeners() {
  // Close WebSocket connection
  if (window.ws) {
    window.ws.close();
  }
  
  // Remove all added event listeners
  // ... (would need to store references to added event listeners for proper cleanup)
}

// Initialize and attach to window unload event for cleanup
window.addEventListener('unload', cleanupEventListeners);
