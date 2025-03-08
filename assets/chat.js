// State management
let messageChain = [];
let currentHead = null;
let currentChatId = null;
let chats = [];
let availableChildren = [];
let runningChildren = [];
let pendingChildMessages = [];
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
    generateButton: getElement('generateButton'),
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
        
        // Log initial state
        console.log('Initial state:', {
            messageChain: messageChain.length,
            currentHead: currentHead,
            currentChatId: currentChatId,
            chats: chats.length,
            pendingChildMessages: pendingChildMessages.length
        });
        
        // Request initial state
        sendWebSocketMessage({ type: 'list_chats' });  // Get available chats
        sendWebSocketMessage({ type: 'get_head' });  // Initial head query
        sendWebSocketMessage({ type: 'get_available_children' });
        sendWebSocketMessage({ type: 'get_running_children' });
        sendWebSocketMessage({ type: 'get_pending_child_messages' });
    };
    
    ws.onclose = () => {
        console.log('WebSocket disconnected');
        updateConnectionStatus('disconnected');
        elements.sendButton.disabled = true;
        elements.generateButton.disabled = true;
        
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
            
            // Enhanced logging for debugging child actor issues
            if (data.type === 'messages_updated' || data.type === 'head') {
                console.log('HEAD UPDATE - Before processing:', {
                    oldHead: currentHead,
                    newHead: data.head,
                    messageChainLength: messageChain.length,
                    messageIDs: messageChain.map(m => m.id)
                });
            } else if (data.type === 'message') {
                console.log('MESSAGE RECEIVED - Details:', {
                    messageId: data.message?.id,
                    messageParents: data.message?.parents,
                    messageType: data.message?.data ? Object.keys(data.message.data)[0] : 'unknown',
                    currentChainLength: messageChain.length
                });
            } else if (data.type === 'children_update') {
                console.log('CHILDREN UPDATE:', {
                    availableCount: data.available_children?.length || 0,
                    runningCount: data.running_children?.length || 0,
                    currentChainLength: messageChain.length,
                    currentHead: currentHead
                });
            } else if (data.type === 'pending_child_messages_update') {
                console.log('PENDING CHILD MESSAGES:', {
                    count: data.pending_messages?.length || 0,
                    messageIds: data.pending_messages?.map(m => m.id) || [],
                    childIds: data.pending_messages?.map(m => m.child_id) || []
                });
            }
            
            handleWebSocketMessage(data);
            
            // Log after processing certain message types
            if (data.type === 'messages_updated' || data.type === 'head' || data.type === 'message') {
                console.log('AFTER HANDLING:', {
                    messageChainLength: messageChain.length,
                    currentHead: currentHead,
                    sortedLength: sortMessageChain().length
                });
            }
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
                    
                    // Enable generate button if we have messages
                    elements.generateButton.disabled = false;
                }
            }
            break;

        case 'message':
            if (data.message) {
                handleNewMessage(data.message);
                // The generate button state is handled in handleNewMessage
            }
            break;
            
        case 'pending_child_messages_update':
            if (data.pending_messages) {
                pendingChildMessages = data.pending_messages;
                renderPendingChildMessages();
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
    console.log('Message chain before handling:', {
        length: messageChain.length,
        ids: messageChain.map(m => m.id),
        currentHead: currentHead
    });
    
    // Remove temporary message if it exists
    const tempMessagesCount = messageChain.filter(m => m.id.startsWith('temp-')).length;
    messageChain = messageChain.filter(m => !m.id.startsWith('temp-'));
    console.log(`Removed ${tempMessagesCount} temporary messages`);
    
    // Add to message chain if not already present
    if (!messageChain.find(m => m.id === message.id)) {
        console.log(`Adding new message to chain: ${message.id}, parents: ${JSON.stringify(message.parents || [])}`);
        // Add the cost to the total only when a new message is received
        if (message.data && message.data.Chat && message.data.Chat.Assistant) {
            const assistant = message.data.Chat.Assistant;
            calculateMessageCost(assistant.usage, true); // Add to total
        }
        messageChain.push(message);
    } else {
        console.log(`Message ${message.id} already exists in chain, skipping`);
    }

    // Request parent messages if needed
    if (message.parents && message.parents.length > 0) {
        console.log(`Message has ${message.parents.length} parents: ${JSON.stringify(message.parents)}`);
        for (const parentId of message.parents) {
            if (!messageChain.find(m => m.id === parentId)) {
                console.log(`Requesting missing parent: ${parentId}`);
                requestMessage(parentId);
            } else {
                console.log(`Parent already in chain: ${parentId}`);
            }
        }
    } else {
        console.log('Message has no parents');
    }

    // Reset waiting state and remove typing indicator
    isWaitingForResponse = false;
    removeTypingIndicator();
    elements.sendButton.disabled = !elements.messageInput.value.trim();
    
    // Enable generate button if we have messages
    elements.generateButton.disabled = (messageChain.length === 0);

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
    
    const isConnected = status === 'connected';
    elements.sendButton.disabled = !isConnected || !elements.messageInput.value.trim();
    elements.generateButton.disabled = !isConnected;
    
    // Disable generate button if there are no messages yet
    if (isConnected && messageChain.length === 0) {
        elements.generateButton.disabled = true;
    }
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
    
    // Enable/disable generate button based on whether we have messages
    elements.generateButton.disabled = (sortedMessages.length === 0);
}

function renderMessage(message) {
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
                        <button class="message-action-button" onclick="copyMessageText('${message.id}')">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                            </svg>
                            <span>Copy Text</span>
                        </button>
                        <div class="action-divider"></div>
                        <button class="message-action-button" onclick="copyMessageId('${message.id}')">
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
            const assistant = msg.Assistant;
            // Determine if this is a short message (less than 100 characters)
            const isShortMessage = assistant.content.length < 100;
            const smallClass = isShortMessage ? 'small' : '';
            
            return `
                <div class="message assistant ${smallClass}" data-message-id="${message.id}">
                    ${formatMessageContent(assistant.content)}
                    <div class="message-actions">
                        <button class="message-action-button" onclick="copyMessageText('${message.id}')">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                            </svg>
                            <span>Copy Text</span>
                        </button>
                        <div class="action-divider"></div>
                        <button class="message-action-button" onclick="copyMessageId('${message.id}')">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                                <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
                            </svg>
                            <span>Copy ID</span>
                        </button>
                    </div>
                    <div class="message-metadata">
                        <div class="metadata-item">
                            <span class="metadata-label">Model:</span> ${assistant.model}
                        </div>
                        <div class="metadata-item">
                            <span class="metadata-label">Tokens:</span> ${assistant.usage.input_tokens} in / ${assistant.usage.output_tokens} out
                        </div>
                        <div class="metadata-item">
                            <span class="metadata-label">Cost:</span> ${calculateMessageCost(assistant.usage, false)}
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
        
        // Determine if this is a short text message (less than 100 characters)
        const isShortMessage = childMsg.text && childMsg.text.length < 100;
        const smallClass = isShortMessage && !childMsg.html ? 'small' : '';
        
        // Check if HTML content exists
        if (childMsg.html) {
            return `
                <div class="child-message ${smallClass}" data-message-id="${message.id}">
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
                        <div class="message-actions">
                            <button class="message-action-button" onclick="copyMessageText('${message.id}')">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                </svg>
                                <span>Copy Text</span>
                            </button>
                            <div class="action-divider"></div>
                            <button class="message-action-button" onclick="copyMessageId('${message.id}')">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                                    <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
                                </svg>
                                <span>Copy ID</span>
                            </button>
                        </div>
                        <div class="child-html-content">${sanitizeHTML(childMsg.html)}</div>
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
        } else if (childMsg.text && childMsg.text.trim() !== '') {
            return `
                <div class="child-message ${smallClass}" data-message-id="${message.id}">
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
                        <div class="message-actions">
                            <button class="message-action-button" onclick="copyMessageText('${message.id}')">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                </svg>
                                <span>Copy Text</span>
                            </button>
                            <div class="action-divider"></div>
                            <button class="message-action-button" onclick="copyMessageId('${message.id}')">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                                    <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
                                </svg>
                                <span>Copy ID</span>
                            </button>
                        </div>
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

// Pending Child Messages handling
function renderPendingChildMessages() {
    // Create a pending messages container if it doesn't exist
    let pendingContainer = document.getElementById('pendingMessagesContainer');
    if (!pendingContainer) {
        pendingContainer = document.createElement('div');
        pendingContainer.id = 'pendingMessagesContainer';
        pendingContainer.className = 'pending-messages-container';
        // Insert it before the input container
        const inputContainer = document.querySelector('.input-container');
        if (inputContainer && inputContainer.parentNode) {
            inputContainer.parentNode.insertBefore(pendingContainer, inputContainer);
        } else {
            // Fallback to append to messages container
            elements.messagesContainer.appendChild(pendingContainer);
        }
    }
    
    // Sort pending messages by timestamp (oldest first)
    const sortedMessages = [...pendingChildMessages].sort((a, b) => a.timestamp - b.timestamp);
    
    // Clear container and add the header if we have pending messages
    if (sortedMessages.length > 0) {
        pendingContainer.innerHTML = `
            <div class="pending-messages-header">
                <h3>Pending Child Messages (${sortedMessages.length})</h3>
                <div class="pending-action-buttons">
                    <button id="selectAllPendingBtn" class="select-all-button">Select All</button>
                    <button id="unselectAllPendingBtn" class="unselect-all-button">Unselect All</button>
                </div>
            </div>
            <div class="pending-messages-list">
                ${sortedMessages.map(renderPendingChildMessage).join('')}
            </div>
        `;
        pendingContainer.style.display = 'block';
        
        // Add event listeners for select/unselect all buttons
        document.getElementById('selectAllPendingBtn').addEventListener('click', () => toggleAllPendingMessages(true));
        document.getElementById('unselectAllPendingBtn').addEventListener('click', () => toggleAllPendingMessages(false));
    } else {
        pendingContainer.style.display = 'none';
        pendingContainer.innerHTML = '';
    }
}

function renderPendingChildMessage(message) {
    // Determine if HTML content should be shown
    const hasHtml = message.html && message.html.trim() !== '';
    const messageContent = hasHtml ? sanitizeHTML(message.html) : formatMessageContent(message.text);
    
    return `
        <div class="pending-child-message ${message.selected ? 'selected' : ''}" data-message-id="${message.id}">
            <div class="pending-child-header">
                <div class="pending-child-info">
                    <span class="child-actor-id">${message.child_id}</span>
                    <div class="pending-message-actions">
                        <label class="toggle-container">
                            <input type="checkbox" class="toggle-inclusion" 
                                ${message.selected ? 'checked' : ''}
                                onchange="togglePendingMessage('${message.id}', this.checked)">
                            <span class="toggle-label">Include</span>
                        </label>
                        <button class="remove-pending-btn" onclick="removePendingMessage('${message.id}')">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M18 6L6 18M6 6l12 12"></path>
                            </svg>
                            Remove
                        </button>
                    </div>
                </div>
            </div>
            <div class="pending-child-content">
                ${hasHtml ? 
                `<div class="child-html-content">${messageContent}</div>` : 
                `<div class="message-content">${messageContent}</div>`}
            </div>
        </div>
    `;
}

// Toggle a specific pending message's inclusion state
function togglePendingMessage(messageId, selected) {
    console.log(`Toggling message ${messageId} to ${selected}`);
    sendWebSocketMessage({
        type: 'toggle_pending_child_message',
        message_id: messageId,
        selected: selected
    });
}

// Toggle all pending messages' inclusion state
function toggleAllPendingMessages(selected) {
    console.log(`Toggling all pending messages to ${selected}`);
    pendingChildMessages.forEach(message => {
        if (message.selected !== selected) {
            togglePendingMessage(message.id, selected);
        }
    });
}

// Remove a pending message
function removePendingMessage(messageId) {
    console.log(`Removing pending message ${messageId}`);
    sendWebSocketMessage({
        type: 'remove_pending_child_message',
        message_id: messageId
    });
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
                <button class="actor-button stop-button" onclick="stopActor('${actor.actor_id}')>
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
function sanitizeHTML(html) {
    // Basic HTML sanitization to prevent XSS
    // This is a simple implementation - consider using a library like DOMPurify in production
    if (!html) return '';

    // Create a temporary element
    const tempDiv = document.createElement('div');
    tempDiv.innerHTML = html;

    // Remove potentially dangerous elements and attributes
    const dangerous = ['script', 'iframe', 'object', 'embed', 'form'];
    dangerous.forEach(tag => {
        const elements = tempDiv.getElementsByTagName(tag);
        while (elements.length > 0) {
            elements[0].parentNode.removeChild(elements[0]);
        }
    });

    // Remove dangerous attributes from all elements
    const allElements = tempDiv.getElementsByTagName('*');
    for (let i = 0; i < allElements.length; i++) {
        const element = allElements[i];
        const attrs = element.attributes;
        for (let j = attrs.length - 1; j >= 0; j--) {
            const attr = attrs[j];
            if (attr.name.startsWith('on') || attr.name === 'href' && attr.value.startsWith('javascript:')) {
                element.removeAttribute(attr.name);
            }
        }
    }

    return tempDiv.innerHTML;
}

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
    console.log('Sorting message chain:', {
        chainLength: messageChain.length,
        currentHead: currentHead
    });
    
    // Create a map for fast lookups
    const messagesById = {};
    messageChain.forEach(msg => {
        messagesById[msg.id] = msg;
    });
    
    // Track visited messages to handle potential cycles
    const visited = new Set();
    const result = [];
    const missingParents = new Set();
    
    // For topological sort - start with all head nodes (nodes with no parents)
    // In our DAG traversal, we'll use a recursive DFS approach
    function processMessage(message, level = 0) {
        console.log(`Processing message: ${message.id} at level ${level}`);
        if (visited.has(message.id)) {
            console.log(`- Already visited ${message.id}, skipping`);
            return;
        }
        visited.add(message.id);
        
        // Process parents first (recursively)
        if (message.parents && message.parents.length > 0) {
            console.log(`- Message ${message.id} has ${message.parents.length} parents`);
            for (const parentId of message.parents) {
                const parent = messagesById[parentId];
                if (parent) {
                    console.log(`- Processing parent: ${parentId}`);
                    processMessage(parent, level + 1);
                } else {
                    console.log(`- MISSING PARENT: ${parentId} for message ${message.id}`);
                    missingParents.add(parentId);
                }
            }
        } else {
            console.log(`- Message ${message.id} has no parents`);
        }
        
        // Add this message to the result
        result.push(message);
    }
    
    // Find the head message (latest message)
    if (currentHead && messagesById[currentHead]) {
        console.log(`Starting traversal from head: ${currentHead}`);
        processMessage(messagesById[currentHead]);
    } else {
        console.log('No current head or head not found in message chain');
        // Without a clear head, process all messages
        console.log('Processing all messages as fallback');
        messageChain.forEach(msg => {
            if (!visited.has(msg.id)) {
                processMessage(msg);
            }
        });
    }
    
    if (missingParents.size > 0) {
        console.warn('MISSING PARENTS DETECTED:', Array.from(missingParents));
    }
    
    console.log(`Sorted message chain: ${result.length} messages`);
    return result;
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
    
    // Reset pending child messages when switching chats
    pendingChildMessages = [];
    renderPendingChildMessages();
    
    sendWebSocketMessage({
        type: 'switch_chat',
        chat_id: chatId
    });
    
    // Reset message chain - will be reloaded from server
    messageChain = [];
    currentHead = null;
    
    // Disable generate button until messages are loaded
    elements.generateButton.disabled = true;
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
    console.log(`Starting actor: ${manifestName}`, {
        currentState: {
            messageChainLength: messageChain.length,
            currentHead: currentHead,
            pendingChildMessages: pendingChildMessages.length
        }
    });
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
    
    console.log('Sending user message:', {
        messageLength: content.length,
        messageChainLength: messageChain.length,
        currentHead: currentHead,
        pendingChildMessages: pendingChildMessages.length
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
    });
}

function generateLlmResponse() {
    if (!ws || ws.readyState !== WebSocket.OPEN || isWaitingForResponse) {
        return;
    }
    
    console.log('Generating LLM response:', {
        messageChainLength: messageChain.length,
        currentHead: currentHead,
        pendingChildMessages: pendingChildMessages.length,
        sortedChainLength: sortMessageChain().length
    });
    
    // Add typing indicator
    addTypingIndicator();
    scrollToBottom();
    
    // Set waiting state
    isWaitingForResponse = true;
    elements.sendButton.disabled = true;
    elements.generateButton.disabled = true;
    
    // Send the generate request
    console.log('Sending WebSocket message to generate LLM response');
    sendWebSocketMessage({
        type: 'generate_llm_response'
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

// Check if mobile view and collapse sidebar if needed
function checkMobileView() {
    const isMobile = window.innerWidth <= 768;
    
    if (isMobile && elements.chatSidebar) {
        elements.chatSidebar.classList.add('collapsed');
        if (elements.expandChatSidebarButton) {
            elements.expandChatSidebarButton.classList.add('visible');
        }
    }
}

// Event listeners
document.addEventListener('DOMContentLoaded', () => {
    // Initialize WebSocket
    connectWebSocket();
    
    // Auto-resize textarea and update button states
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
    
    // One event handler for all keyboard shortcuts
    elements.messageInput.addEventListener('keydown', (event) => {
        console.log(`Key pressed: ${event.key}, Shift: ${event.shiftKey}, Ctrl: ${event.ctrlKey}, Meta: ${event.metaKey}`);
        
        if (event.key === 'Enter') {
            // Shift+Enter to send message
            if (event.shiftKey) {
                event.preventDefault();
                console.log('Sending message with Shift+Enter');
                sendMessage();
            }
            // Ctrl+Enter or Cmd+Enter to generate response
            else if (event.ctrlKey || event.metaKey) {
                event.preventDefault();
                console.log('Generating response with Ctrl/Cmd+Enter');
                generateLlmResponse();
            }
            // Normal Enter just allows the newline (default behavior)
        }
    });
    
    // Button click handlers
    elements.sendButton.addEventListener('click', sendMessage);
    elements.generateButton.addEventListener('click', generateLlmResponse);
    
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
    
    // Check for mobile view on load
    checkMobileView();
    
    // Handle window resize
    window.addEventListener('resize', checkMobileView);
});

// Copy message functions
function copyMessageText(messageId) {
    // Find the message in the message chain
    const message = messageChain.find(m => m.id === messageId);
    if (!message) return;
    
    // Extract the content based on the message type
    let content = '';
    if (message.data.Chat) {
        if (message.data.Chat.User) {
            content = message.data.Chat.User.content;
        } else if (message.data.Chat.Assistant) {
            content = message.data.Chat.Assistant.content;
        }
    } else if (message.data.ChildMessage) {
        content = message.data.ChildMessage.text;
    }
    
    // Copy to clipboard
    navigator.clipboard.writeText(content)
        .then(() => {
            showCopySuccess('Text copied to clipboard');
        })
        .catch(err => {
            console.error('Failed to copy text: ', err);
            showError('Failed to copy text');
        });
}

function copyMessageId(messageId) {
    // Copy the message ID to clipboard
    navigator.clipboard.writeText(messageId)
        .then(() => {
            showCopySuccess('ID copied to clipboard');
        })
        .catch(err => {
            console.error('Failed to copy ID: ', err);
            showError('Failed to copy ID');
        });
}

function showCopySuccess(message) {
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

// Cleanup
window.addEventListener('unload', () => {
    if (ws) {
        ws.close();
    }
});
