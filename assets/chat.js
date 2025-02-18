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

    messageArea.innerHTML = renderMessageTree(messages);
    
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

    // Find root messages
    const rootMessages = messages.filter(msg => 
        !msg.parent || !messages.find(m => m.id === msg.parent)
    );

    // Render message tree
    return `
        <div class="message-container">
            ${rootMessages.map(msg => renderMessageTreeNode(msg, messageChildren)).join('')}
        </div>
    `;
}

function renderMessageTreeNode(message, messageChildren) {
    if (!message) {
        return '';
    }

    // Skip rendering Rollup type messages directly
    if (message.type === 'Rollup') {
        return '';
    }

    // Get children of this message
    const children = messageChildren.get(message.id) || [];
    const hasActorResponses = children.some(child => child.type === 'Rollup');

    const actorResponsesHtml = hasActorResponses ? 
        renderActorResponsesSection(message.id, children) : '';

    const childMessagesHtml = children
        .filter(child => child.type !== 'Rollup')
        .map(child => renderMessageTreeNode(child, messageChildren))
        .join('');

    return `
        <div class="message-tree">
            <div class="message ${message.role} ${message.id === selectedMessageId ? 'selected' : ''}" 
                 data-id="${message.id}">
                ${formatMessage(message.content)}
                <div class="message-actions">
                    <button class="message-action-button copy-button">
                        Copy ID
                    </button>
                </div>
            </div>
            ${actorResponsesHtml}
            ${childMessagesHtml}
        </div>
    `;
}

function renderActorResponsesSection(messageId, children) {
    const actorResponses = children.filter(child => child.type === 'Rollup');
    const totalResponses = actorResponses.reduce((sum, rollup) => {
        return sum + (rollup.child_responses ? rollup.child_responses.length : 0);
    }, 0);
    
    if (totalResponses === 0) {
        return '';
    }

    const responsesHtml = actorResponses.map(rollup => {
        if (!rollup.child_responses) {
            return '';
        }
        return rollup.child_responses.map(response => `
            <div class="actor-response">
                <div class="actor-response-content">
                    <div class="actor-response-header">
                        <span class="actor-name">Actor: ${response.child_id}</span>
                    </div>
                    ${formatMessage(response.content || 'No response content')}
                </div>
            </div>
        `).join('');
    }).join('');

    return `
        <div class="actor-responses-wrapper" data-message-id="${messageId}">
            <div class="actor-responses-indicator" onclick="toggleActorResponses('${messageId}')">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path d="M9 18l6-6-6-6" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                ${totalResponses} actor response${totalResponses !== 1 ? 's' : ''}
            </div>
            <div class="actor-responses" id="actor-responses-${messageId}">
                ${responsesHtml}
            </div>
        </div>
    `;
}

function toggleActorResponses(messageId) {
    const wrapper = document.querySelector(`.actor-responses-wrapper[data-message-id="${messageId}"]`);
    if (wrapper) {
        const indicator = wrapper.querySelector('.actor-responses-indicator');
        const responses = wrapper.querySelector('.actor-responses');
        if (indicator && responses) {
            indicator.classList.toggle('expanded');
            responses.classList.toggle('expanded');
        }
    }
}

// Message formatting
function formatMessage(content) {
    if (!content) return '';
    
    // First escape HTML and convert newlines to <br>
    let text = content.toString()
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;")
        .replace(/\n/g, '<br>');
    
    // Format code blocks
    text = text.replace(/```([^`]+)```/g, (match, code) => `<pre><code>${code}</code></pre>`);
    
    // Format inline code
    text = text.replace(/`([^`]+)`/g, (match, code) => `<code>${code}</code>`);
    
    return text;
}

// Message actions
function handleMessageClick(event) {
    const messageElement = event.target.closest('.message');
    if (!messageElement) return;

    // Don't trigger if clicking action button
    if (event.target.closest('.message-action-button')) return;

    const messageId = messageElement.dataset.id;
    selectedMessageId = selectedMessageId === messageId ? null : messageId;
    renderMessages([...messageCache.values()]);
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
    const statusElement = document.querySelector('.connection-status');
    if (statusElement) {
        statusElement.className = 'connection-status ' + status;
        statusElement.textContent = status.charAt(0).toUpperCase() + status.slice(1);
    }
}

function connectWebSocket() {
    console.log("Connecting to WebSocket...");
    updateConnectionStatus('connecting');
    ws = new WebSocket(WEBSOCKET_URL);
    
    ws.onopen = () => {
        console.log("WebSocket connected");
        updateConnectionStatus('connected');
        reconnectAttempts = 0;
        sendWebSocketMessage({ type: 'get_messages' });
        initializeChildPanel();
    };
    
    ws.onclose = () => {
        console.log("WebSocket disconnected");
        updateConnectionStatus('disconnected');
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttempts++;
            setTimeout(connectWebSocket, 1000 * Math.min(reconnectAttempts, 30));
        }
    };
    
    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateConnectionStatus('disconnected');
    };
    
    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            console.log("WebSocket message received:", data);
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
        console.log("WebSocket not connected, can't send message");
        updateConnectionStatus('disconnected');
    }
}

function handleWebSocketMessage(data) {
    switch(data.type) {
        case 'message_update':
            if (data.messages) {
                console.log("Updating messages:", data.messages);
                data.messages.forEach(msg => messageCache.set(msg.id, msg));
                const allMessages = Array.from(messageCache.values());
                renderMessages(allMessages);
                updateHeadId(allMessages);
            }
            break;
        case 'children_update':
            console.log("Children update received:", data);
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

// Child panel management
function initializeChildPanel() {
    console.log("Initializing child panel...");
    if (!ws || ws.readyState !== WebSocket.OPEN) {
        console.log("WebSocket not ready, retrying in 1s...");
        setTimeout(initializeChildPanel, 1000);
        return;
    }
    
    console.log("Requesting available children");
    sendWebSocketMessage({
        type: 'get_available_children'
    });
    
    console.log("Requesting running children");
    sendWebSocketMessage({
        type: 'get_running_children'
    });
}

function startChild(manifestName) {
    console.log("Starting child actor:", manifestName);
    sendWebSocketMessage({
        type: 'start_child',
        manifest_name: manifestName
    });
}

function stopChild(actorId) {
    console.log("Stopping child actor:", actorId);
    sendWebSocketMessage({
        type: 'stop_child',
        actor_id: actorId
    });
}

function updateHeadId(messages) {
    const headElement = document.querySelector('.head-id');
    if (messages && messages.length > 0) {
        const lastMessage = messages[messages.length - 1];
        headElement.textContent = `Head: ${lastMessage.id.slice(0, 8)}...`;
    } else {
        headElement.textContent = 'Head: None';
    }
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
                                <span class="actor-name">${child.name || child.manifest_name}</span>
                                ${child.description ? `<span class="actor-description">${child.description}</span>` : ''}
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

    const collapseButton = panel.querySelector('.collapse-button');
    if (collapseButton) {
        collapseButton.addEventListener('click', () => {
            console.log("Toggle child panel collapse");
            panel.classList.toggle('collapsed');
        });
    }
}

// Message sending
async function sendMessage() {
    const messageInput = document.getElementById('messageInput');
    const text = messageInput.value.trim();
    if (!text) return;

    try {
        const tempMsg = {
            role: 'user',
            content: text,
            id: 'temp-' + Date.now(),
            type: 'Message',
            parent: null
        };
        
        messageCache.set(tempMsg.id, tempMsg);
        renderMessages([...messageCache.values()]);

        messageInput.value = '';
        messageInput.style.height = '2.5rem';
        messageInput.focus();

        sendWebSocketMessage({
            type: 'send_message',
            content: text
        });
    } catch (error) {
        console.error('Error sending message:', error);
        alert('Failed to send message. Please try again.');
    }
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    console.log("Initializing application...");
    const messageArea = document.getElementById('messageArea');
    const messageInput = document.getElementById('messageInput');
    
    // Connect WebSocket
    connectWebSocket();
    
    // Auto-resize textarea
    messageInput.addEventListener('input', () => {
        messageInput.style.height = 'auto';
        messageInput.style.height = Math.min(messageInput.scrollHeight, 200) + 'px';
    });

    // Handle message sending
    messageInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });

    // Global shortcut for focusing input
    document.addEventListener('keydown', (event) => {
        if (event.key === '/' && document.activeElement !== messageInput) {
            event.preventDefault();
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
    console.log("Page unloading, cleaning up...");
    if (ws) {
        ws.close();
    }
});
