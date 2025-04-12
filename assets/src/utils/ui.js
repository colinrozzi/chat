// UI utilities
import { elements } from './elements.js';
import { chats, currentChatId, totalCost, totalInputTokens, totalOutputTokens, totalMessages } from '../components/app.js';

// Update connection status display
export function updateConnectionStatus(status) {
  elements.connectionStatus.className = 'connection-status ' + status;
  elements.connectionStatus.innerHTML = `
    <div class="status-indicator"></div>
    <span>${status.charAt(0).toUpperCase() + status.slice(1)}</span>
  `;
  
  const isConnected = status === 'connected';
  elements.sendButton.disabled = !isConnected || !elements.messageInput.value.trim();
  elements.generateButton.disabled = !isConnected;
  
  // Disable generate button if there are no messages yet
  if (isConnected && window.messageChain.length === 0) {
    elements.generateButton.disabled = true;
  }
}

// Show error message
export function showError(message) {
  console.error('Error:', message);
  const errorDiv = document.createElement('div');
  errorDiv.className = 'error-message';
  errorDiv.innerHTML = `
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="10"></circle>
      <line x1="12" y1="8" x2="12" y2="12"></line>
      <line x1="12" y1="16" x2="12.01" y2="16"></line>
    </svg>
    ${message}
  `;
  elements.messagesContainer.prepend(errorDiv);
  setTimeout(() => errorDiv.remove(), 5000);
  
  // Hide loading overlay if it's visible
  elements.loadingOverlay.classList.remove('visible');
}

// Show success message
export function showSuccess(message) {
  console.log('Success:', message);
  const successDiv = document.createElement('div');
  successDiv.className = 'success-message';
  successDiv.innerHTML = `
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
      <polyline points="22 4 12 14.01 9 11.01"></polyline>
    </svg>
    ${message}
  `;
  elements.messagesContainer.prepend(successDiv);
  setTimeout(() => successDiv.remove(), 2000);
}

// Show copy success notification
export function showCopySuccess(message) {
  const successDiv = document.createElement('div');
  successDiv.className = 'success-message';
  successDiv.innerHTML = `
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
      <polyline points="22 4 12 14.01 9 11.01"></polyline>
    </svg>
    ${message}
  `;
  elements.messagesContainer.prepend(successDiv);
  setTimeout(() => successDiv.remove(), 2000);
}

// Update current chat name in the UI
export function updateCurrentChatName() {
  const currentChat = window.chats.find(chat => chat.id === window.currentChatId);
  console.log('Updating current chat name:', currentChat);
  
  if (currentChat) {
    // Safely handle potential null/undefined name
    const displayName = currentChat.name || 'Unnamed Chat';
    elements.currentChatName.textContent = displayName;
    
    // Add a title attribute for tooltip on hover
    elements.currentChatName.title = `${displayName} (Click to rename)`;
    
    // Make the chat name editable
    elements.currentChatName.onclick = () => {
      if (window.currentChatId) {
        window.showRenameChat(window.currentChatId, elements.currentChatName.textContent);
      }
    };
    
    // Add a visual indicator that the name is editable
    elements.currentChatName.classList.add('editable');
  } else {
    console.log('No current chat selected');
    elements.currentChatName.textContent = 'No Chat Selected';
    elements.currentChatName.title = 'No Chat Selected';
    elements.currentChatName.onclick = null;
    elements.currentChatName.classList.remove('editable');
  }
}

// Update stats display
export function updateStatsDisplay() {
  // Update the cost in the header
  updateTotalCostDisplay();
  
  // Update the stats in the controls sidebar
  if (elements.statsMessageCount) {
    elements.statsMessageCount.textContent = window.totalMessages;
  }
  
  if (elements.statsTokenCount) {
    elements.statsTokenCount.textContent = `${window.totalInputTokens} in / ${window.totalOutputTokens} out`;
  }
  
  if (elements.statsTotalCost) {
    elements.statsTotalCost.textContent = `${window.totalCost.toFixed(4)}`;
  }
}

// Update the total cost display
export function updateTotalCostDisplay() {
  const costElement = document.querySelector('.cost-value');
  if (costElement) {
    costElement.textContent = `${window.totalCost.toFixed(4)}`;
  }
}

// Scroll to the bottom of the messages container
export function scrollToBottom() {
  elements.messagesContainer.scrollTop = elements.messagesContainer.scrollHeight;
}

// Make the chat controls sidebar collapsed by default on desktop
export function initializeSidebars() {
  // On initial load, collapse the controls sidebar on desktop
  if (window.innerWidth > 768 && elements.chatControlsSidebar) {
    elements.chatControlsSidebar.classList.add('collapsed');
    if (elements.expandChatControlsButton) {
      elements.expandChatControlsButton.classList.add('visible');
    }
  }
}

// Toggle chat sidebar
export function toggleChatSidebar() {
  if (!elements.chatSidebar) return;
  
  elements.chatSidebar.classList.toggle('collapsed');
  
  if (elements.expandChatSidebarButton) {
    if (elements.chatSidebar.classList.contains('collapsed')) {
      elements.expandChatSidebarButton.classList.add('visible');
    } else {
      elements.expandChatSidebarButton.classList.remove('visible');
    }
  }
}

// Toggle chat controls sidebar
export function toggleChatControlsSidebar() {
  if (!elements.chatControlsSidebar) return;
  
  elements.chatControlsSidebar.classList.toggle('collapsed');
  
  if (elements.expandChatControlsButton) {
    if (elements.chatControlsSidebar.classList.contains('collapsed')) {
      elements.expandChatControlsButton.classList.add('visible');
    } else {
      elements.expandChatControlsButton.classList.remove('visible');
    }
  }
}

// Toggle section
export function toggleSection(sectionId) {
  const section = document.getElementById(sectionId).closest('.section');
  section.classList.toggle('collapsed');
}

// Render empty state for messages container
export function renderEmptyState() {
  return `
    <div class="empty-state">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
      </svg>
      <p>No messages yet</p>
      <p class="text-sm">Start a conversation!</p>
    </div>
  `;
}
