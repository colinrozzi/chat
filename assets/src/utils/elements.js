// DOM Elements utilities

// Get DOM elements with safety (returns null if element doesn't exist)
function getElement(id) {
  return document.getElementById(id);
}

// Export DOM elements for use across modules
export const elements = {
  messageInput: getElement('messageInput'),
  sendButton: getElement('sendButton'),
  generateButton: getElement('generateButton'),
  messagesContainer: getElement('messagesContainer'),
  connectionStatus: getElement('connectionStatus'),
  loadingOverlay: getElement('loadingOverlay'),
  headId: getElement('headId'),
  chatSidebar: getElement('chatSidebar'),
  chatList: getElement('chatList'),
  currentChatName: getElement('currentChatName'),
  newChatButton: getElement('newChatButton'),
  branchChatButton: getElement('branchChatButton'),
  collapseChatSidebarButton: getElement('collapseChatSidebarButton'),
  expandChatSidebarButton: getElement('expandChatSidebarButton'),
  // Chat Controls Sidebar Elements
  chatControlsSidebar: getElement('chatControlsSidebar'),
  collapseChatControlsButton: getElement('collapseChatControlsButton'),
  expandChatControlsButton: getElement('expandChatControlsButton'),
  controlsModelSelector: getElement('controlsModelSelector'),
  modelContextWindow: getElement('modelContextWindow'),
  modelInfo: getElement('modelInfo'),
  // Stats elements
  statsMessageCount: getElement('statsMessageCount'),
  statsTokenCount: getElement('statsTokenCount'),
  statsTotalCost: getElement('statsTotalCost')
};
