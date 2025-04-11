// Chat component handling rendering and interaction
import { messageChain, currentChatId, chats, currentHead } from '../app.js';
import { elements } from '../utils/elements.js';
import { sortMessageChain } from '../utils/message-chain.js';
import { formatMessageContent } from '../utils/formatters.js';
import { renderEmptyState, showError, showSuccess } from '../utils/ui.js';
import { getModelMaxTokens } from '../utils/models.js';
import { sendWebSocketMessage } from '../services/websocket.js';
import { scrollToBottom } from '../utils/ui.js';

// Render messages in the chat container
export function renderMessages() {
  const sortedMessages = sortMessageChain();
  elements.messagesContainer.innerHTML = sortedMessages.length ? 
    sortedMessages.map(renderMessage).join('') :
    renderEmptyState();
  
  // Enable/disable generate button based on whether we have messages
  elements.generateButton.disabled = (sortedMessages.length === 0);
}

// Render a single message
export function renderMessage(message) {
  console.log('Rendering message:', message, 'Message data:', JSON.stringify(message.data, null, 2));
  if (message.data.Chat) {
    const msg = message.data.Chat;
    // Handle the new Message enum structure
    if (msg.User) {
      // Determine if this is a short message (less than 80 characters)
      const isShortMessage = msg.User.content.length < 80;
      const smallClass = isShortMessage ? 'small' : '';
      
      return `
        <div class="message user ${smallClass}" data-message-id="${message.id}">
          ${formatMessageContent(msg.User.content)}
          <div class="message-actions">
            <button class="message-action-button" onclick="window.copyMessageText('${message.id}')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span>Copy Text</span>
            </button>
            <div class="action-divider"></div>
            <button class="message-action-button" onclick="window.copyMessageId('${message.id}')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
              </svg>
              <span>Copy ID</span>
            </button>
          </div>
        </div>
      `;
    } else if (msg.Assistant) {
      const assistantMsg = msg.Assistant;
      let content, model, usage, stopReason, providerName, costDisplay;
      
      // Handle the nested structure (Claude, Gemini, or OpenRouter)
      if (assistantMsg.Claude) {
        const claude = assistantMsg.Claude;
        content = claude.content;
        model = claude.model;
        usage = claude.usage;
        stopReason = claude.stop_reason;
        providerName = "Claude";
        
        // Calculate cost for Claude
        if (claude.input_cost_per_million_tokens !== null && claude.output_cost_per_million_tokens !== null) {
          const inputCost = (claude.usage.input_tokens / 1000000) * claude.input_cost_per_million_tokens;
          const outputCost = (claude.usage.output_tokens / 1000000) * claude.output_cost_per_million_tokens;
          costDisplay = (inputCost + outputCost).toFixed(4);
        } else {
          costDisplay = "Unknown";
        }
      } else if (assistantMsg.Gemini) {
        const gemini = assistantMsg.Gemini;
        content = gemini.content;
        model = gemini.model;
        usage = gemini.usage;
        stopReason = gemini.finish_reason;
        providerName = "Gemini";
        
        // Calculate cost for Gemini
        if (gemini.input_cost_per_million_tokens !== null && gemini.output_cost_per_million_tokens !== null) {
          const inputCost = (gemini.usage.prompt_tokens / 1000000) * gemini.input_cost_per_million_tokens;
          const outputCost = (gemini.usage.completion_tokens / 1000000) * gemini.output_cost_per_million_tokens;
          costDisplay = (inputCost + outputCost).toFixed(4);
        } else {
          costDisplay = "Unknown";
        }
      } else if (assistantMsg.OpenRouter) {
        // Handle OpenRouter messages
        const openrouter = assistantMsg.OpenRouter;
        content = openrouter.content;
        model = openrouter.model;
        usage = openrouter.usage;
        stopReason = openrouter.finish_reason;
        providerName = "OpenRouter";
        
        // Calculate cost for OpenRouter
        if (openrouter.input_cost_per_million_tokens !== null && openrouter.output_cost_per_million_tokens !== null) {
          const inputCost = (openrouter.usage.prompt_tokens / 1000000) * openrouter.input_cost_per_million_tokens;
          const outputCost = (openrouter.usage.completion_tokens / 1000000) * openrouter.output_cost_per_million_tokens;
          costDisplay = (inputCost + outputCost).toFixed(4);
        } else if (openrouter.usage.cost !== null) {
          costDisplay = openrouter.usage.cost.toFixed(4);
        } else {
          costDisplay = "Unknown";
        }
      } else {
        // Fallback for older message structure
        content = assistantMsg.content || "Content unavailable";
        model = assistantMsg.model || "Unknown model";
        usage = assistantMsg.usage || { input_tokens: 0, output_tokens: 0 };
        stopReason = assistantMsg.stop_reason || assistantMsg.finish_reason || "Unknown";
        providerName = model?.startsWith("gemini-") ? "Gemini" : "Claude";
        costDisplay = "Unknown";
      }
      
      // Determine if this is a short message (less than 100 characters)
      const isShortMessage = content?.length < 100;
      const smallClass = isShortMessage ? 'small' : '';
      
      return `
        <div class="message assistant ${smallClass}" data-message-id="${message.id}">
          ${formatMessageContent(content)}
          <div class="message-actions">
            <button class="message-action-button" onclick="window.copyMessageText('${message.id}')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span>Copy Text</span>
            </button>
            <div class="action-divider"></div>
            <button class="message-action-button" onclick="window.copyMessageId('${message.id}')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
              </svg>
              <span>Copy ID</span>
            </button>
          </div>
          <div class="message-metadata">
            <div class="metadata-item">
              <span class="metadata-label">Provider:</span> ${providerName}
            </div>
            <div class="metadata-item">
              <span class="metadata-label">Model:</span> ${model}
            </div>
            <div class="metadata-item">
              <span class="metadata-label">Tokens:</span> ${usage ? (usage.input_tokens || usage.prompt_tokens || 0) : 0} in / ${usage ? (usage.output_tokens || usage.completion_tokens || 0) : 0} out of ${getModelMaxTokens(model)}
            </div>
            <div class="metadata-item">
              <span class="metadata-label">Cost:</span> ${costDisplay}
            </div>
            <div class="metadata-item">
              <span class="metadata-label">Stop Reason:</span> ${stopReason}
            </div>
          </div>
        </div>
      `;
    }
  } 
  return '';
}

// Render the chat list in the sidebar
export function renderChatList() {
  // Sort chats by updated_at (newest first) if available, or fallback to sorting by ID
  const sortedChats = [...chats].sort((a, b) => {
    if (a.updated_at && b.updated_at) {
      return b.updated_at - a.updated_at;
    }
    return a.id.localeCompare(b.id);
  });
  
  elements.chatList.innerHTML = sortedChats.length ?
    sortedChats.map(chat => `
      <div class="chat-item ${chat.id === currentChatId ? 'active' : ''}" data-chat-id="${chat.id}">
        <div class="chat-item-name" onclick="window.switchChat('${chat.id}')">${chat.name}</div>
        <div class="chat-item-actions">
          <button class="chat-action rename" onclick="window.showRenameChat('${chat.id}', '${chat.name}')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path>
            </svg>
          </button>
          <button class="chat-action delete" onclick="window.confirmDeleteChat('${chat.id}')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18"></path>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            </svg>
          </button>
        </div>
      </div>
    `).join('') :
    '<div class="empty-state">No chats available</div>';
}

// Create a new chat
export function createNewChat(wsConnection) {
  const chatName = prompt('Enter a name for the new chat:', 'New Chat');
  if (chatName === null) return; // User cancelled
  
  // Validate the chat name
  if (!chatName.trim()) {
    showError('Chat name cannot be empty');
    return;
  }
  
  if (chatName.length > 50) {
    showError('Chat name too long (maximum 50 characters)');
    return;
  }
  
  const trimmedName = chatName.trim();
  console.log(`Creating new chat with name: "${trimmedName}"`);
  
  // Show loading state
  elements.loadingOverlay.classList.add('visible');
  
  // Add a temporary entry to the chats array while we wait for server response
  const tempId = 'temp-' + Date.now();
  const tempChat = {
    id: tempId,
    name: trimmedName,
    isTemporary: true
  };
  
  // Add to chats array and render with loading state
  chats.push(tempChat);
  renderChatList();
  
  // Highlight the temporary chat
  const tempChatElement = document.querySelector(`.chat-item[data-chat-id="${tempId}"] .chat-item-name`);
  if (tempChatElement) {
    tempChatElement.innerHTML += ' <span class="loading-text">(Creating...)</span>';
  }
  
  sendWebSocketMessage({
    type: 'create_chat',
    name: trimmedName,
    starting_head: null // No starting head for a fresh chat
  }, wsConnection);
  
  // Reset message chain for new chat
  messageChain.length = 0;
  window.currentHead = null;
  
  // Clear the messages display immediately
  elements.messagesContainer.innerHTML = renderEmptyState();
  
  // Update the head ID display
  elements.headId.textContent = '';
  
  // Disable generate button for the new empty chat
  elements.generateButton.disabled = true;
  
  // If no server response after 5 seconds, remove temporary chat and show error
  setTimeout(() => {
    // Check if temporary chat still exists (server didn't respond)
    const tempChatStillExists = chats.some(c => c.id === tempId);
    if (tempChatStillExists) {
      // Remove temporary chat from array
      chats.splice(chats.findIndex(c => c.id === tempId), 1);
      renderChatList();
      showError('Failed to create chat. Server did not respond.');
    }
    elements.loadingOverlay.classList.remove('visible');
  }, 5000);
}

// Branch from the current chat to create a new one
export function branchChat(wsConnection) {
  if (!currentHead) {
    showError('Cannot branch from an empty chat');
    return;
  }
  
  const chatName = prompt('Enter a name for the branched chat:', 'Branch of current chat');
  if (chatName === null) return; // User cancelled
  
  sendWebSocketMessage({
    type: 'create_chat',
    name: chatName,
    starting_head: currentHead // Start from current head
  }, wsConnection);
  
  // Message chain will be loaded when the server notifies us of the new chat
}

// Switch to a different chat
export function switchChat(chatId, wsConnection) {
  if (chatId === currentChatId) return; // Already on this chat
  
  // Reset pending child messages when switching chats
  
  sendWebSocketMessage({
    type: 'switch_chat',
    chat_id: chatId
  }, wsConnection);
  
  // Reset message chain - will be reloaded from server
  messageChain.length = 0;
  window.currentHead = null;
  window.lastUsedModelId = null;
  
  // Clear the messages display immediately
  elements.messagesContainer.innerHTML = renderEmptyState();
  
  // Update the head ID display
  elements.headId.textContent = '';
  
  // Disable generate button until messages are loaded
  elements.generateButton.disabled = true;
}

// Show prompt to rename a chat
export function showRenameChat(chatId, currentName) {
  // Decode HTML entities in the current name for the prompt
  const decodedCurrentName = currentName.replace(/&amp;/g, '&')
                                       .replace(/&lt;/g, '<')
                                       .replace(/&gt;/g, '>')
                                       .replace(/&quot;/g, '"');
  
  const newName = prompt('Enter a new name for the chat:', decodedCurrentName);
  if (newName === null || newName === decodedCurrentName) return; // User cancelled or unchanged
  
  // Validate the new name (e.g., not empty, not too long)
  if (!newName.trim()) {
    showError('Chat name cannot be empty');
    return;
  }
  
  if (newName.length > 50) {
    showError('Chat name too long (maximum 50 characters)');
    return;
  }
  
  console.log(`Renaming chat ${chatId} from "${currentName}" to "${newName.trim()}"`); // Trim the name
  
  // Show visual feedback that rename is in progress
  const chatElement = document.querySelector(`.chat-item[data-chat-id="${chatId}"] .chat-item-name`);
  if (chatElement) {
    const originalText = chatElement.textContent;
    chatElement.textContent = 'Renaming...';
    
    // Restore original text after a delay if the rename operation takes too long
    setTimeout(() => {
      if (chatElement.textContent === 'Renaming...') {
        chatElement.textContent = originalText;
      }
    }, 3000);
  }
  
  sendWebSocketMessage({
    type: 'rename_chat',
    chat_id: chatId,
    name: newName.trim() // Ensure we trim the name
  }, window.ws);
}

// Confirm and delete a chat
export function confirmDeleteChat(chatId) {
  // Find the chat to display its name
  const chat = chats.find(c => c.id === chatId);
  const chatName = chat ? chat.name : 'this chat';
  
  const confirmed = confirm(`Are you sure you want to delete "${chatName}"?\n\nThis action cannot be undone.`);
  if (!confirmed) return;
  
  sendWebSocketMessage({
    type: 'delete_chat',
    chat_id: chatId
  }, window.ws);
}

// Send a user message
export function sendMessage(wsConnection) {
  const content = elements.messageInput.value.trim();
  
  if (!content || !wsConnection || wsConnection.readyState !== WebSocket.OPEN || window.isWaitingForResponse) {
    return;
  }
  
  console.log('Sending user message:', {
    messageLength: content.length,
    messageChainLength: messageChain.length,
    currentHead: currentHead,
  });
  
  // Create temporary message object for optimistic rendering
  const tempMessage = {
    id: 'temp-' + Date.now(),
    data: {
      Chat: {
        User: {
          content: content
        }
      }
    }
  };
  
  // Add to message chain and render immediately
  messageChain.push(tempMessage);
  console.log('Added temporary message to chain, new length:', messageChain.length);
  renderMessages();
  scrollToBottom();
  
  // Set waiting state
  elements.messageInput.value = '';
  elements.messageInput.style.height = 'auto';
  elements.messageInput.focus();
  elements.sendButton.disabled = true;
  
  // Enable the generate button now that we have a message
  elements.generateButton.disabled = false;
  
  // Send the actual message
  console.log('Sending WebSocket message with user content');
  sendWebSocketMessage({
    type: 'send_message',
    content: content
  }, wsConnection);
}

// Generate an AI response
export function generateLlmResponse(wsConnection, modelId) {
  if (!wsConnection || wsConnection.readyState !== WebSocket.OPEN || window.isWaitingForResponse) {
    return;
  }
  
  // Get the selected model from the controls sidebar if not provided
  if (!modelId) {
    modelId = elements.controlsModelSelector?.value;
  }
  
  // Store this as the most recently used model
  if (modelId) {
    window.lastUsedModelId = modelId;
    console.log(`Set lastUsedModelId to: ${window.lastUsedModelId}`);
  }
  
  console.log('Generating LLM response:', {
    model: modelId,
    messageChainLength: messageChain.length,
    currentHead: currentHead,
    sortedChainLength: sortMessageChain().length
  });
  
  // Set waiting state
  window.isWaitingForResponse = true;
  elements.sendButton.disabled = true;
  elements.generateButton.disabled = true;
  
  // Add typing indicator
  import('../utils/typing-indicator.js').then(({ addTypingIndicator }) => {
    addTypingIndicator();
    scrollToBottom();
  });
  
  // Send the generate request with model ID
  console.log('Sending WebSocket message to generate LLM response');
  sendWebSocketMessage({
    type: 'generate_llm_response',
    model_id: modelId
  }, wsConnection);
}
