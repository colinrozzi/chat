// State management
let messageChain = [];
let currentHead = null;
let currentChatId = null;
let chats = [];
let availableChildren = [];
let runningChildren = [];
let ws = null;
let reconnectAttempts = 0;
let totalCost = 0;
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_DELAY = 1000;

// DOM Elements
// Get DOM elements with safety (returns null if element doesn't exist)
function getElement(id) {
    return document.getElementById(id);
}

const elements = {
    messageInput: getElement('messageInput'),
    sendButton: getElement('sendButton'),
    messagesContainer: getElement('messagesContainer'),
    connectionStatus: getElement('connectionStatus'),
    loadingOverlay: getElement('loadingOverlay'),
    actorPanel: getElement('actorPanel'),
    collapseButton: getElement('collapseButton'),
    expandButton: getElement('expandButton'),
    availableActors: getElement('availableActors'),
    runningActors: getElement('runningActors'),
    headId: getElement('headId'),
    chatSidebar: getElement('chatSidebar'),
    chatList: getElement('chatList'),
    currentChatName: getElement('currentChatName'),
    newChatButton: getElement('newChatButton'),
    branchChatButton: getElement('branchChatButton'),
    collapseChatSidebarButton: getElement('collapseChatSidebarButton'),
    expandChatSidebarButton: getElement('expandChatSidebarButton')
};

// WebSocket setup
function connectWebSocket() {
    console.log('Connecting to WebSocket...');
    updateConnectionStatus('connecting');
    
    ws = new WebSocket(`ws://localhost:{{WEBSOCKET_PORT}}/ws`);
    
    ws.onopen = () => {
        console.log('WebSocket connected');
        updateConnectionStatus('connected');
        reconnectAttempts = 0;
        
        // Request initial state
        sendWebSocketMessage({ type: 'list_chats' });  // Get available chats
        sendWebSocketMessage({ type: 'get_head' });  // Initial head query
        sendWebSocketMessage({ type: 'get_available_children' });
        sendWebSocketMessage({ type: 'get_running_children' });
    };
    
    ws.onclose = () => {
        console.log('WebSocket disconnected');
        updateConnectionStatus('disconnected');
        elements.sendButton.disabled = true;
        
        // Disconnection handling
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttempts++;
            setTimeout(connectWebSocket, RECONNECT_DELAY * Math.min(reconnectAttempts, 30));
        }
    };
    
    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            console.log('Received WebSocket message:', data);
            handleWebSocketMessage(data);
        } catch (error) {
            console.error('WebSocket message processing error:', error);
            console.error('Raw message:', event.data);
            showError('Failed to process server message');
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        showError('Connection error occurred');
    };
}

// Message handling
function handleWebSocketMessage(data) {
    console.log('Received message:', data);

    switch (data.type) {
        case 'children_update':
            if (data.available_children) {
                availableChildren = data.available_children;
            }
            if (data.running_children) {
                runningChildren = data.running_children;
            }
            renderActorPanels();
            break;

        case 'messages_updated':
        case 'head':
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
                    requestMessage(data.head);
                }
            }
            break;

        case 'message':
            if (data.message) {
                handleNewMessage(data.message);
            }
            break;
            
        case 'chats_update':
            if (data.chats) {
                chats = data.chats;
                if (data.current_chat_id) {
                    currentChatId = data.current_chat_id;
                }
                renderChatList();
                updateCurrentChatName();
            }
            break;
            
        case 'chat_created':
            if (data.chat) {
                // Add to chats array if not already present
                if (!chats.find(c => c.id === data.chat.id)) {
                    chats.push(data.chat);
                }
                // Update current chat ID
                currentChatId = data.chat.id;
                renderChatList();
                updateCurrentChatName();
            }
            break;
            
        case 'chat_renamed':
            if (data.chat) {
                // Update chat in the array
                const index = chats.findIndex(c => c.id === data.chat.id);
                if (index !== -1) {
                    chats[index] = { ...chats[index], ...data.chat };
                    renderChatList();
                    updateCurrentChatName();
                }
            }
            break;
            
        case 'chat_deleted':
            if (data.chat_id) {
                // Remove chat from array
                chats = chats.filter(c => c.id !== data.chat_id);
                renderChatList();
                updateCurrentChatName();
            }
            break;
            
        case 'error':
            showError(data.message || 'An error occurred');
            break;
    }
}

function handleNewMessage(message) {
    console.log('Handling new message:', message);
    
    // Remove temporary message if it exists
    messageChain = messageChain.filter(m => !m.id.startsWith('temp-'));
    
    // Add to message chain if not already present
    if (!messageChain.find(m => m.id === message.id)) {
        // Add the cost to the total only when a new message is received
        if (message.data && message.data.Chat && message.data.Chat.Assistant) {
            const assistant = message.data.Chat.Assistant;
            calculateMessageCost(assistant.usage, true); // Add to total
        }
        messageChain.push(message);
    }

    // Request parent message if needed
    if (message.parent && !messageChain.find(m => m.id === message.parent)) {
        requestMessage(message.parent);
    }

    // Reset waiting state and remove typing indicator
    isWaitingForResponse = false;
    removeTypingIndicator();
    elements.sendButton.disabled = !elements.messageInput.value.trim();

    renderMessages();
    scrollToBottom();
}

// UI Updates
function updateConnectionStatus(status) {
    elements.connectionStatus.className = 'connection-status ' + status;
    elements.connectionStatus.innerHTML = `
        <div class="status-indicator"></div>
        <span>${status.charAt(0).toUpperCase() + status.slice(1)}</span>
    `;
    elements.sendButton.disabled = status !== 'connected';
}

function showError(message) {
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
}

function renderMessages() {
    const sortedMessages = sortMessageChain();
    elements.messagesContainer.innerHTML = sortedMessages.length ? 
        sortedMessages.map(renderMessage).join('') :
        renderEmptyState();
}

function renderMessage(message) {
    console.log('Rendering message:', message, '\nMessage data:', JSON.stringify(message.data, null, 2));
    if (message.data.Chat) {
        const msg = message.data.Chat;
        // Handle the new Message enum structure
        if (msg.User) {
            return `
                <div class="message user">
                    ${formatMessageContent(msg.User.content)}
                </div>
            `;
        } else if (msg.Assistant) {
            const assistant = msg.Assistant;
            return `
                <div class="message assistant">
                    ${formatMessageContent(assistant.content)}
                    <div class="message-metadata">
                        <div class="metadata-item">
                            <span class="metadata-label">Model:</span> ${assistant.model}
                        </div>
                        <div class="metadata-item">
                            <span class="metadata-label">Tokens:</span> ${assistant.usage.input_tokens} in / ${assistant.usage.output_tokens} out
                        </div>
                        <div class="metadata-item">
                            <span class="metadata-label">Cost:</span> $${calculateMessageCost(assistant.usage, false)}
                        </div>
                        <div class="metadata-item">
                            <span class="metadata-label">Stop Reason:</span> ${assistant.stop_reason}
                        </div>
                    </div>
                </div>
            `;
        }
    } else if (message.data.ChildMessage) {
        // Handle individual child message
        const childMsg = message.data.ChildMessage;
        if (childMsg.text && childMsg.text.trim() !== '') {
            return `
                <div class="child-message">
                    <div class="child-message-header" onclick="toggleChildMessage(this)">
                        <div class="child-header">Actor: ${childMsg.child_id}</div>
                        ${Object.keys(childMsg.data || {}).length > 0 ? `
                            <button class="child-data-toggle" onclick="toggleChildData('child-${message.id}-${childMsg.child_id}')">
                                <span>View Data</span>
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M9 18l6-6-6-6"/>
                                </svg>
                            </button>
                        ` : ''}
                    </div>
                    <div class="child-message-content">
                        ${formatMessageContent(childMsg.text)}
                        ${Object.keys(childMsg.data || {}).length > 0 ? `
                            <div id="child-${message.id}-${childMsg.child_id}" class="child-data">
                                <div class="child-data-content">
                                    ${formatJsonData(childMsg.data)}
                                </div>
                            </div>
                        ` : ''}
                    </div>
                </div>
            `;
        }
    }
    return '';
}

function formatJsonData(data) {
    try {
        // Convert the data to a formatted string with 2-space indentation
        return JSON.stringify(data, null, 2)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    } catch (error) {
        console.error('Error formatting JSON data:', error);
        return 'Error displaying data';
    }
}

function toggleChildData(messageId) {
    const container = document.getElementById(messageId);
    const content = container.querySelector('.child-data-content');
    const toggle = container.parentElement.querySelector('.child-data-toggle');
    
    content.classList.toggle('expanded');
    toggle.classList.toggle('expanded');
    
    // Update toggle text
    const toggleText = toggle.querySelector('span');
    toggleText.textContent = content.classList.contains('expanded') ? 'Hide Data' : 'View Data';
}

function renderEmptyState() {
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

function renderActorPanels() {
    // Available Actors
    elements.availableActors.innerHTML = availableChildren.length ?
        availableChildren.map(actor => `
            <div class="actor-card">
                <div class="actor-name">${actor.name || actor.manifest_name}</div>
                ${actor.description ? `<div class="actor-description">${actor.description}</div>` : ''}
                <button class="actor-button start-button" onclick="startActor('${actor.manifest_name}')">
                    Start
                </button>
            </div>
        `).join('') :
        '<div class="empty-state">No available actors</div>';

    // Running Actors
    elements.runningActors.innerHTML = runningChildren.length ?
        runningChildren.map(actor => `
            <div class="actor-card">
                <div class="actor-name">${actor.manifest_name}</div>
                <div class="actor-id">${actor.actor_id}</div>
                <button class="actor-button stop-button" onclick="stopActor('${actor.actor_id}')">
                    Stop
                </button>
            </div>
        `).join('') :
        '<div class="empty-state">No running actors</div>';
}

// Cost calculation
function calculateMessageCost(usage, addToTotal = false) {
    const INPUT_COST_PER_MILLION = 3;
    const OUTPUT_COST_PER_MILLION = 15;
    
    const inputCost = (usage.input_tokens / 1000000) * INPUT_COST_PER_MILLION;
    const outputCost = (usage.output_tokens / 1000000) * OUTPUT_COST_PER_MILLION;
    const messageCost = inputCost + outputCost;
    
    // Update total cost only when explicitly requested
    if (addToTotal) {
        totalCost += messageCost;
        updateTotalCostDisplay();
    }
    
    // Format to 4 decimal places
    return messageCost.toFixed(4);
}

function updateTotalCostDisplay() {
    const costElement = document.querySelector('.cost-value');
    if (costElement) {
        costElement.textContent = `$${totalCost.toFixed(4)}`;
    }
}

// Utility functions
function formatMessageContent(content) {
    if (!content) return '';
    
    // Escape HTML
    let text = content
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    
    // Format code blocks
    text = text.replace(/```([^`]+)```/g, (_, code) => 
        `<pre><code>${code}</code></pre>`
    );
    
    // Format inline code
    text = text.replace(/`([^`]+)`/g, (_, code) => 
        `<code>${code}</code>`
    );
    
    // Convert newlines to <br>
    text = text.replace(/\n/g, '<br>');
    
    return text;
}

function sortMessageChain() {
    const sorted = [];
    const visited = new Set();

    function addMessage(id) {
        if (!id || visited.has(id)) return;
        
        const message = messageChain.find(m => m.id === id);
        if (!message) return;

        if (message.parent) {
            addMessage(message.parent);
        }

        if (!visited.has(id)) {
            sorted.push(message);
            visited.add(id);
        }
    }

    if (currentHead) {
        addMessage(currentHead);
    } else {
        messageChain
            .sort((a, b) => (!a.parent && b.parent) ? -1 : (a.parent && !b.parent) ? 1 : 0)
            .forEach(message => addMessage(message.id));
    }

    return sorted;
}

// Chat management functions
function renderChatList() {
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
                <div class="chat-item-name" onclick="switchChat('${chat.id}')">${chat.name}</div>
                <div class="chat-item-actions">
                    <button class="chat-action rename" onclick="showRenameChat('${chat.id}', '${chat.name}')">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path>
                        </svg>
                    </button>
                    <button class="chat-action delete" onclick="confirmDeleteChat('${chat.id}')">
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

function updateCurrentChatName() {
    const currentChat = chats.find(chat => chat.id === currentChatId);
    elements.currentChatName.textContent = currentChat ? currentChat.name : 'No Chat Selected';
    
    // Make the chat name editable
    elements.currentChatName.onclick = () => {
        if (currentChatId) {
            showRenameChat(currentChatId, elements.currentChatName.textContent);
        }
    };
}

function createNewChat() {
    const chatName = prompt('Enter a name for the new chat:', 'New Chat');
    if (chatName === null) return; // User cancelled
    
    sendWebSocketMessage({
        type: 'create_chat',
        name: chatName,
        starting_head: null // No starting head for a fresh chat
    });
    
    // Reset message chain for new chat
    messageChain = [];
    currentHead = null;
}

function branchChat() {
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
    });
    
    // Message chain will be loaded when the server notifies us of the new chat
}

function switchChat(chatId) {
    if (chatId === currentChatId) return; // Already on this chat
    
    sendWebSocketMessage({
        type: 'switch_chat',
        chat_id: chatId
    });
    
    // Reset message chain - will be reloaded from server
    messageChain = [];
    currentHead = null;
}

function showRenameChat(chatId, currentName) {
    const newName = prompt('Enter a new name for the chat:', currentName);
    if (newName === null || newName === currentName) return; // User cancelled or unchanged
    
    sendWebSocketMessage({
        type: 'rename_chat',
        chat_id: chatId,
        name: newName
    });
}

function confirmDeleteChat(chatId) {
    // Find the chat to display its name
    const chat = chats.find(c => c.id === chatId);
    const chatName = chat ? chat.name : 'this chat';
    
    const confirmed = confirm(`Are you sure you want to delete "${chatName}"?\n\nThis action cannot be undone.`);
    if (!confirmed) return;
    
    sendWebSocketMessage({
        type: 'delete_chat',
        chat_id: chatId
    });
}

// WebSocket communication
function sendWebSocketMessage(message) {
    if (ws?.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(message));
    } else {
        showError('Not connected to server');
    }
}

function requestMessage(messageId) {
    sendWebSocketMessage({
        type: 'get_message',
        message_id: messageId
    });
}

function scrollToBottom() {
    elements.messagesContainer.scrollTop = elements.messagesContainer.scrollHeight;
}

// Actor management
function startActor(manifestName) {
    sendWebSocketMessage({
        type: 'start_child',
        manifest_name: manifestName
    });
}

function stopActor(actorId) {
    sendWebSocketMessage({
        type: 'stop_child',
        actor_id: actorId
    });
}

function sendTestMessage(actorId) {
    const timestamp = new Date().toLocaleTimeString();
    sendWebSocketMessage({
        type: 'child_message',
        child_id: actorId,
        text: `This is a test message from actor ${actorId} at ${timestamp}`,
        data: {
            timestamp: timestamp,
            type: 'test'
        }
    });
}

function sendChildMessage(childId, text, data = {}) {
    sendWebSocketMessage({
        type: 'child_message',
        child_id: childId,
        text: text,
        data: data
    });
}

// Message input handling
let isWaitingForResponse = false;

function addTypingIndicator() {
    const typingIndicator = document.createElement('div');
    typingIndicator.className = 'typing-indicator';
    typingIndicator.id = 'typingIndicator';
    typingIndicator.innerHTML = `
        <div class="typing-dots">
            <div class="typing-dot"></div>
            <div class="typing-dot"></div>
            <div class="typing-dot"></div>
        </div>
    `;
    elements.messagesContainer.appendChild(typingIndicator);
    scrollToBottom();
}

function removeTypingIndicator() {
    const indicator = document.getElementById('typingIndicator');
    if (indicator) {
        indicator.remove();
    }
}

function sendMessage() {
    const content = elements.messageInput.value.trim();
    
    if (!content || !ws || ws.readyState !== WebSocket.OPEN || isWaitingForResponse) {
        return;
    }
    
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
    renderMessages();
    
    // Add typing indicator
    addTypingIndicator();
    scrollToBottom();
    
    // Set waiting state
    isWaitingForResponse = true;
    elements.messageInput.value = '';
    elements.messageInput.style.height = 'auto';
    elements.messageInput.focus();
    elements.sendButton.disabled = true;
    
    // Send the actual message
    sendWebSocketMessage({
        type: 'send_message',
        content: content
    });
}

// Toggle chat sidebar
function toggleChatSidebar() {
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

// Toggle child message content
function toggleChildMessage(header) {
    const content = header.nextElementSibling;
    content.classList.toggle('expanded');
    header.classList.toggle('collapsed');
}

// Toggle section
function toggleSection(sectionId) {
    const section = document.getElementById(sectionId).closest('.section');
    section.classList.toggle('collapsed');
}

// Event listeners
document.addEventListener('DOMContentLoaded', () => {
    // Initialize WebSocket
    connectWebSocket();
    
    // Auto-resize textarea
    elements.messageInput.addEventListener('input', () => {
        elements.messageInput.style.height = 'auto';
        elements.messageInput.style.height = Math.min(elements.messageInput.scrollHeight, 120) + 'px';
        elements.sendButton.disabled = !elements.messageInput.value.trim();
    });
    
    // Add keyboard shortcut for focusing message input
    document.addEventListener('keydown', (event) => {
        if (event.key === '\\') {
            event.preventDefault(); // Prevent the \ from being typed
            elements.messageInput.focus();
        }
    });
    
    // Send message handlers
    elements.messageInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
    
    elements.sendButton.addEventListener('click', sendMessage);
    
    // Chat sidebar toggle handlers
    if (elements.collapseChatSidebarButton) {
        elements.collapseChatSidebarButton.addEventListener('click', toggleChatSidebar);
    }
    if (elements.expandChatSidebarButton) {
        elements.expandChatSidebarButton.addEventListener('click', toggleChatSidebar);
    }
    
    // Actor panel toggle handlers
    if (elements.collapseButton) {
        elements.collapseButton.addEventListener('click', () => {
            if (elements.actorPanel) elements.actorPanel.classList.add('collapsed');
            if (elements.expandButton) elements.expandButton.classList.add('visible');
        });
    }
    
    if (elements.expandButton) {
        elements.expandButton.addEventListener('click', () => {
            if (elements.actorPanel) elements.actorPanel.classList.remove('collapsed');
            if (elements.expandButton) elements.expandButton.classList.remove('visible');
        });
    }
    
    // New chat button
    if (elements.newChatButton) {
        elements.newChatButton.addEventListener('click', createNewChat);
    }
    
    // Branch chat button
    if (elements.branchChatButton) {
        elements.branchChatButton.addEventListener('click', branchChat);
    }
});

// Cleanup
window.addEventListener('unload', () => {
    if (ws) {
        ws.close();
    }
});