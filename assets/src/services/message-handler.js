// Message handling service for WebSocket messages
import { 
  messageChain, currentHead, currentChatId, chats, models,
  lastUsedModelId, isWaitingForResponse
} from '../components/app.js';
import { elements } from '../utils/elements.js';
import { showError, showSuccess, updateCurrentChatName } from '../utils/ui.js';
import { renderChatList, renderMessages } from '../components/chat.js';
import { updateModelSelector, populateModelSelector, updateModelInfo } from '../components/model-selector.js';
import { findLastUsedModel } from '../utils/models.js';
import { requestMessage } from './websocket.js';
import { handleNewMessage } from '../components/message.js';
import { removeTypingIndicator } from '../utils/typing-indicator.js';
import { scrollToBottom } from '../utils/ui.js';

// Handle incoming WebSocket messages
export function handleWebSocketMessage(data, wsConnection) {
  console.log('Processing message:', data);

  switch (data.type) {
    case 'messages_updated':
    case 'head':
      handleHeadUpdate(data, wsConnection);
      break;

    case 'message':
      if (data.message) {
        handleNewMessage(data.message, wsConnection);
      }
      break;
      
    case 'chats_update':
      handleChatsUpdate(data);
      break;
      
    case 'chat_created':
      handleChatCreated(data);
      break;
      
    case 'chat_renamed':
      handleChatRenamed(data);
      break;
      
    case 'chat_deleted':
      handleChatDeleted(data);
      break;
      
    case 'models_list':
      handleModelsList(data);
      break;
      
    case 'error':
      handleError(data);
      break;
  }
}

// Handle head update messages
function handleHeadUpdate(data, wsConnection) {
  if (data.current_chat_id && data.current_chat_id !== currentChatId) {
    currentChatId = data.current_chat_id;
    updateCurrentChatName();
    renderChatList();
  }
  
  if (data.head) {
    // Check if head has changed
    if (data.head !== currentHead) {
      console.log(`Head updated: ${currentHead} -> ${data.head}`);
      currentHead = data.head;
      elements.headId.textContent = `Head: ${data.head.substring(0, 8)}...`;
      requestMessage(data.head, wsConnection);
      
      // After getting the new head, find the last used model
      findLastUsedModel();
      
      // Enable generate button if we have messages
      elements.generateButton.disabled = false;
    }
  }
}

// Handle chats update
function handleChatsUpdate(data) {
  if (data.chats) {
    chats = data.chats;
    if (data.current_chat_id) {
      currentChatId = data.current_chat_id;
    }
    renderChatList();
    updateCurrentChatName();
  }
}

// Handle chat created event
function handleChatCreated(data) {
  if (data.chat) {
    console.log('Received chat_created event:', data.chat);
    
    // Remove any temporary chats first
    chats = chats.filter(c => !c.isTemporary);
    
    // Add to chats array if not already present
    if (!chats.find(c => c.id === data.chat.id)) {
      chats.push(data.chat);
      console.log(`Added new chat to chats array: ${data.chat.id} - ${data.chat.name}`);
    } else {
      console.log(`Chat already exists in array, updating: ${data.chat.id}`);
      // Update existing chat with new data
      const index = chats.findIndex(c => c.id === data.chat.id);
      if (index !== -1) {
        chats[index] = { 
          ...chats[index], 
          ...data.chat,
          name: data.chat.name || chats[index].name, // Ensure name is preserved
          icon: data.chat.icon !== undefined ? data.chat.icon : chats[index].icon
        };
      }
    }
    
    // Update current chat ID
    currentChatId = data.chat.id;
    
    // Ensure the message display is cleared for the new chat
    if (messageChain.length > 0) {
      messageChain = [];
      currentHead = null;
      elements.messagesContainer.innerHTML = renderEmptyState();
      elements.headId.textContent = '';
      elements.generateButton.disabled = true;
    }
    
    // Hide loading overlay
    elements.loadingOverlay.classList.remove('visible');
    
    renderChatList();
    updateCurrentChatName();
    
    // Show success notification
    showSuccess(`Chat "${data.chat.name}" created successfully`);
  } else {
    console.error('Received chat_created event without chat data');
    showError('Error creating chat: Invalid response from server');
  }
}

// Handle chat renamed event
function handleChatRenamed(data) {
  if (data.chat) {
    console.log('Received chat_renamed event:', data.chat);
    // Update chat in the array
    const index = chats.findIndex(c => c.id === data.chat.id);
    if (index !== -1) {
      // Store the old name for logging
      const oldName = chats[index].name;
      
      // Properly preserve all existing properties while updating only what changed
      chats[index] = { 
        ...chats[index], 
        name: data.chat.name || chats[index].name,
        icon: data.chat.icon !== undefined ? data.chat.icon : chats[index].icon
      };
      console.log(`Updated chat in array: ${chats[index].id} renamed from "${oldName}" to "${chats[index].name}"`);
      
      renderChatList();
      updateCurrentChatName();
      
      // Show success notification
      showSuccess(`Chat renamed to "${chats[index].name}" successfully`);
    } else {
      console.warn('Received rename event for non-existent chat ID:', data.chat.id);
      showError('Error: Chat not found');
    }
  } else {
    console.error('Received chat_renamed event without chat data');
    showError('Error renaming chat: Invalid response from server');
  }
}

// Handle chat deleted event
function handleChatDeleted(data) {
  if (data.chat_id) {
    // Remove chat from array
    chats = chats.filter(c => c.id !== data.chat_id);
    renderChatList();
    updateCurrentChatName();
  }
}

// Handle models list
function handleModelsList(data) {
  if (data.models) {
    models = data.models;
    populateModelSelector();
    // Update the model info in the sidebar
    updateModelInfo();
  }
}

// Handle error messages
function handleError(data) {
  console.error('Error from server:', data);
  // Check if this is a chat operation error and provide more context
  if (data.error_code === 'rename_chat_failed') {
    showError(data.message || 'Failed to rename chat');
  } else if (data.error_code === 'create_chat_failed') {
    showError(data.message || 'Failed to create chat');
  } else {
    showError(data.message || 'An error occurred');
  }
  
  // Hide loading overlay if it's visible
  elements.loadingOverlay.classList.remove('visible');
}
