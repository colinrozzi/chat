// State management
let messageCache = new Map();
let ws = null;
let reconnectAttempts = 0;
let selectedMessageId = null;
const MAX_RECONNECT_ATTEMPTS = 5;
const WEBSOCKET_URL = 'ws://localhost:{{WEBSOCKET_PORT}}/';

// Child actor management
let availableChildren = [];
let runningChildren = [];

function initializeChildPanel() {
    console.log("Initializing child panel, requesting available and running children");
    // Request available children list
    sendWebSocketMessage({
        type: 'get_available_children'
    });

    // Request running children list
    sendWebSocketMessage({
        type: 'get_running_children'
    });
}

function startChild(manifestName) {
    console.log("Attempting to start child actor:", manifestName);
    sendWebSocketMessage({
        type: 'start_child',
        manifest_name: manifestName
    });
}

function stopChild(actorId) {
    console.log("Attempting to stop child actor:", actorId);
    sendWebSocketMessage({
        type: 'stop_child',
        actor_id: actorId
    });
}

function renderChildPanel() {
    console.log("Rendering child panel");
    console.log("Available children:", availableChildren);
    console.log("Running children:", runningChildren);
    
    const panel = document.getElementById('childPanel');
    if (!panel) {
        console.error("Child panel element not found!");
        return;
    }
    
    panel.innerHTML = `
        <div class="panel-header">
            <h2>Actor Management</h2>
            <button class="collapse-button">×</button>
        </div>
        <div class="panel-content">
            <div class="section">
                <h3>Available Actors</h3>
                ${availableChildren.length ? 
                    availableChildren.map(child => `
                        <div class="actor-item">
                            <div class="actor-info">
                                <span class="actor-name">${child.name}</span>
                                <span class="actor-description">${child.description}</span>
                            </div>
                            <button class="start-button" onclick="startChild('${child.manifest_name}')">
                                Start
                            </button>
                        </div>
                    `).join('') 
                    : '<div class="empty-state">No available actors</div>'
                }
            </div>
            <div class="section">
                <h3>Running Actors</h3>
                ${runningChildren.length ?
                    runningChildren.map(child => `
                        <div class="actor-item">
                            <div class="actor-info">
                                <span class="actor-name">${child.manifest_name}</span>
                                <span class="actor-id">ID: ${child.actor_id}</span>
                            </div>
                            <button class="stop-button" onclick="stopChild('${child.actor_id}')">
                                Stop
                            </button>
                        </div>
                    `).join('')
                    : '<div class="empty-state">No running actors</div>'
                }
            </div>
        </div>
    `;

    // Add collapse button handler
    const collapseButton = panel.querySelector('.collapse-button');
    if (collapseButton) {
        collapseButton.addEventListener('click', () => {
            console.log("Toggle child panel collapse");
            panel.classList.toggle('collapsed');
        });
    }
}

// Updated message rendering in chat.js:
function renderMessageTree(messages) {
    // Build a map of message IDs to their children
    const messageChildren = new Map();
    messages.forEach(msg => {
        if (msg.parent) {
            if (!messageChildren.has(msg.parent)) {
                messageChildren.set(msg.parent, []);
            }
            messageChildren.get(msg.parent).push(msg);
        }
    });

    // Find root messages (ones without parents or whose parents don't exist)
    const rootMessages = messages.filter(msg => 
        !msg.parent || !messages.find(m => m.id === msg.parent)
    );

    // Recursively render message trees
    return rootMessages.map(msg => renderMessageTreeNode(msg, messageChildren)).join('');
}

// Update renderMessageTreeNode to handle both message and rollup types
function renderMessageTreeNode(message, messageChildren) {
    // Check if this is a rollup or a regular message
    if (message.type === 'Rollup') {
        const rollup = message;
        return `
            <div class="message-tree">
                ${rollup.child_responses.length > 0 ? `
                    <div class="child-responses">
                        ${rollup.child_responses.map(response => `
                            <div class="child-response">
                                <div class="child-response-header">
                                    <span>Actor: ${response.child_id}</span>
                                </div>
                            </div>
                        `).join('')}
                    </div>
                ` : ''}
                ${messageChildren.get(message.id)?.map(child => 
                    renderMessageTreeNode(child, messageChildren)
                ).join('') || ''}
            </div>
        `;
    } else if (message.type === 'Message') {
        const regularMessage = message;
        return `
            <div class="message-tree">
                <div class="message ${regularMessage.role} ${regularMessage.id === selectedMessageId ? 'selected' : ''}" 
                     data-id="${regularMessage.id}">
                    ${formatMessage(regularMessage.content)}
                    <div class="message-actions">
                        <button class="message-action-button copy-button">
                            Copy ID
                        </button>
                    </div>
                </div>
                ${messageChildren.get(regularMessage.id)?.map(child => 
                    renderMessageTreeNode(child, messageChildren)
                ).join('') || ''}
            </div>
        `;
    }
    return '';
}

// UI Elements
const messageInput = document.getElementById('messageInput');
const messageArea = document.getElementById('messageArea');
const loadingOverlay = document.getElementById('messageLoading');

// Message rendering
function renderMessages(messages, isTyping = false) {
    console.log("Rendering messages, typing indicator:", isTyping);
    
    if (messages.length === 0 && !isTyping) {
        messageArea.innerHTML = `
            <div class="empty-state">
                No messages yet.<br>Start the conversation!
            </div>
        `;
        return;
    }

    messageArea.innerHTML = `
        <div class="message-container">
            ${renderMessageTree(messages)}
            ${isTyping ? `
                <div class="typing-indicator">
                    <span></span>
                    <span></span>
                    <span></span>
                </div>
            ` : ''}
        </div>
    `;

    // Set up event listeners for messages
    messageArea.querySelectorAll('.message').forEach(messageElement => {
        messageElement.addEventListener('click', handleMessageClick);
        
        const copyButton = messageElement.querySelector('.copy-button');
        if (copyButton) {
            copyButton.addEventListener('click', (event) => {
                event.stopPropagation();
                copyMessageId(messageElement.dataset.id, copyButton);
            });
        }
    });

    messageArea.scrollTop = messageArea.scrollHeight;
}

// Auto-resize textarea
function adjustTextareaHeight() {
    messageInput.style.height = 'auto';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 200) + 'px';
}

messageInput.addEventListener('input', adjustTextareaHeight);

// Message formatting
function formatMessage(content) {
    // First escape HTML and convert newlines to <br>
    let text = escapeHtml(content).replace(/\n/g, '<br>');
    
    // Format code blocks
    text = text.replace(/```([^`]+)```/g, (match, code) => `<pre><code>${code}</code></pre>`);
    
    // Format inline code
    text = text.replace(/`([^`]+)`/g, (match, code) => `<code>${code}</code>`);
    
    return text;
}

function escapeHtml(unsafe) {
    return unsafe
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

// Message actions
function handleMessageClick(event) {
    const messageElement = event.target.closest('.message');
    if (!messageElement) return;

    // Don't trigger if clicking action button
    if (event.target.closest('.message-action-button')) return;

    const messageId = messageElement.dataset.id;
    
    // If clicking the same message, deselect it
    if (selectedMessageId === messageId) {
        selectedMessageId = null;
    } else {
        selectedMessageId = messageId;
    }
    renderMessages([...messageCache.values()], false);
}

function copyMessageId(messageId, button) {
    navigator.clipboard.writeText(messageId)
        .then(() => {
            const originalText = button.textContent;
            button.textContent = 'Copied!';
            setTimeout(() => {
                button.textContent = originalText;
            }, 1000);
        })
        .catch(err => {
            console.error('Failed to copy message ID:', err);
            alert('Failed to copy message ID');
        });
}

// WebSocket connection management
function updateConnectionStatus(status) {
    console.log("Updating connection status:", status);
    const statusElement = document.querySelector('.connection-status');
    if (!statusElement) {
        console.error("Connection status element not found");
        return;
    }
    
    statusElement.className = 'connection-status ' + status;
    
    switch(status) {
        case 'connected':
            statusElement.textContent = 'Connected';
            break;
        case 'disconnected':
            statusElement.textContent = 'Disconnected';
            break;
        case 'connecting':
            statusElement.textContent = 'Connecting...';
            break;
    }
}

function connectWebSocket() {
    console.log("Attempting WebSocket connection to:", WEBSOCKET_URL);
    updateConnectionStatus('connecting');
    
    ws = new WebSocket(WEBSOCKET_URL);
    
    ws.onopen = () => {
        console.log("WebSocket connection established");
        updateConnectionStatus('connected');
        reconnectAttempts = 0;
        
        // Request initial messages
        console.log("Requesting initial messages");
        sendWebSocketMessage({
            type: 'get_messages'
        });
        
        // Request child info
        initializeChildPanel();
    };
    
    ws.onclose = () => {
        console.log("WebSocket connection closed");
        updateConnectionStatus('disconnected');
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttempts++;
            const delay = 1000 * Math.min(reconnectAttempts, 30);
            console.log(`Attempting reconnect in ${delay}ms (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`);
            setTimeout(connectWebSocket, delay);
        } else {
            console.error("Max reconnection attempts reached");
        }
    };
    
    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateConnectionStatus('disconnected');
    };
    
    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            console.log("Received WebSocket message:", data);
            handleWebSocketMessage(data);
        } catch (error) {
            console.error('Error parsing WebSocket message:', error);
        }
    };
}

function sendWebSocketMessage(message) {
    console.log("Sending WebSocket message:", message);
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(message));
    } else {
        console.warn('WebSocket not connected, message not sent');
        updateConnectionStatus('disconnected');
    }
}

function handleWebSocketMessage(data) {
    console.log("Processing WebSocket message type:", data.type);
    
    switch(data.type) {
        case 'message_update':
            if (data.messages) {
                console.log("Updating message cache with new messages:", data.messages);
                // Update message cache with new messages
                data.messages.forEach(msg => {
                    messageCache.set(msg.id, msg);
                });
                
                // Remove any temporary messages
                for (const [id, msg] of messageCache.entries()) {
                    if (id.startsWith('temp-')) {
                        console.log("Removing temporary message:", id);
                        messageCache.delete(id);
                    }
                }
                
                // Get all messages in order and render
                const allMessages = Array.from(messageCache.values());
                renderMessages(allMessages, false);
                
                // Update head ID if present
                updateHeadId(allMessages);
            }
            break;
        case 'children_update':
            console.log("Processing children update:", data);
            if (data.available_children) {
                console.log("Updating available children:", data.available_children);
                availableChildren = data.available_children;
            }
            if (data.running_children) {
                console.log("Updating running children:", data.running_children);
                runningChildren = data.running_children;
            }
            renderChildPanel();
            break;
        default:
            console.log("Unknown message type:", data.type);
    }
}

// Update head ID in title
function updateHeadId(messages) {
    console.log("Updating head ID display");
    const headElement = document.querySelector('.head-id');
    if (messages && messages.length > 0) {
        const lastMessage = messages[messages.length - 1];
        headElement.textContent = `Head: ${lastMessage.id.slice(0, 8)}...`;
    } else {
        headElement.textContent = 'Head: None';
    }
}

// Message handling
async function sendMessage() {
    const text = messageInput.value.trim();
    if (!text) return;

    console.log("Attempting to send message:", text);
    const sendButton = document.querySelector('.send-button');

    try {
        messageInput.disabled = true;
        sendButton.disabled = true;

        // Create and show user message immediately
        const userMsg = {
            role: 'user',
            content: text,
            id: 'temp-' + Date.now(),
            parent: null
        };
        console.log("Created temporary message:", userMsg);
        messageCache.set(userMsg.id, userMsg);
        
        // Show messages with typing indicator
        renderMessages([...messageCache.values()], true);

        // Send message to server
        sendWebSocketMessage({
            type: 'send_message',
            content: text
        });

        messageInput.value = '';
        messageInput.style.height = '2.5rem';
        messageInput.focus();
    } catch (error) {
        console.error('Error sending message:', error);
        alert('Failed to send message. Please try again.');
    } finally {
        messageInput.disabled = false;
        sendButton.disabled = false;
    }
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    console.log("Page loaded, initializing application");
    // Connect websocket first
    connectWebSocket();

    // Setup message input handling
    messageInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });

    // Add global keyboard shortcut for focusing the input
    document.addEventListener('keydown', (event) => {
        // Check if user is not already typing in the input
        if (event.key === '/' && document.activeElement !== messageInput) {
            event.preventDefault(); // Prevent the '/' from being typed
            messageInput.focus();
        }
    });
});

// Handle visibility changes
document.addEventListener('visibilitychange', () => {
    console.log("Visibility changed, document hidden:", document.hidden);
    if (!document.hidden && (!ws || ws.readyState !== WebSocket.OPEN)) {
        console.log("Page visible and WebSocket not connected, attempting reconnection");
        connectWebSocket();
    }
});

// Cleanup on page unload
window.addEventListener('unload', () => {
    console.log("Page unloading, closing WebSocket connection");
    if (ws) {
        ws.close();
    }
});
