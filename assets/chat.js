// State management
let messageChain = [];
let currentHead = null;
let availableChildren = [];
let runningChildren = [];
let ws = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_DELAY = 1000;

// DOM Elements
const elements = {
    messageInput: document.getElementById('messageInput'),
    sendButton: document.getElementById('sendButton'),
    messagesContainer: document.getElementById('messagesContainer'),
    connectionStatus: document.getElementById('connectionStatus'),
    loadingOverlay: document.getElementById('loadingOverlay'),
    actorPanel: document.getElementById('actorPanel'),
    collapseButton: document.getElementById('collapseButton'),
    togglePanelButton: document.getElementById('togglePanelButton'),
    availableActors: document.getElementById('availableActors'),
    runningActors: document.getElementById('runningActors')
};

// WebSocket setup
function connectWebSocket() {
    console.log('Connecting to WebSocket...');
    updateConnectionStatus('connecting');
    
    ws = new WebSocket(`ws://localhost:{{WEBSOCKET_PORT}}/`);
    
    ws.onopen = () => {
        console.log('WebSocket connected');
        updateConnectionStatus('connected');
        reconnectAttempts = 0;
        
        // Request initial state
        sendWebSocketMessage({ type: 'get_head' });
        sendWebSocketMessage({ type: 'get_available_children' });
        sendWebSocketMessage({ type: 'get_running_children' });
    };
    
    ws.onclose = () => {
        console.log('WebSocket disconnected');
        updateConnectionStatus('disconnected');
        elements.sendButton.disabled = true;
        
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttempts++;
            setTimeout(connectWebSocket, RECONNECT_DELAY * Math.min(reconnectAttempts, 30));
        }
    };
    
    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleWebSocketMessage(data);
        } catch (error) {
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
            if (data.head) {
                currentHead = data.head;
                requestMessage(data.head);
            }
            break;

        case 'message':
            if (data.message) {
                handleNewMessage(data.message);
            }
            break;
    }
}

function handleNewMessage(message) {
    // Add to message chain if not already present
    if (!messageChain.find(m => m.id === message.id)) {
        messageChain.push(message);
    }

    // Request parent message if needed
    if (message.parent && !messageChain.find(m => m.id === message.parent)) {
        requestMessage(message.parent);
    }

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
    if (message.data.Chat) {
        const { role, content } = message.data.Chat;
        return `
            <div class="message ${role}">
                ${formatMessageContent(content)}
            </div>
        `;
    } else if (message.data.Child) {
        const { child_id, text } = message.data.Child;
        return `
            <div class="message child">
                <div class="child-header">Actor: ${child_id}</div>
                ${formatMessageContent(text)}
            </div>
        `;
    }
    return '';
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

function scrollToBottom() {
    elements.messagesContainer.scrollTop = elements.messagesContainer.scrollHeight;
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

// Message input handling
function sendMessage() {
    const content = elements.messageInput.value.trim();
    
    if (!content || !ws || ws.readyState !== WebSocket.OPEN) {
        return;
    }
    
    sendWebSocketMessage({
        type: 'send_message',
        content: content
    });
    
    elements.messageInput.value = '';
    elements.messageInput.style.height = 'auto';
    elements.messageInput.focus();
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
    
    // Send message handlers
    elements.messageInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
    
    elements.sendButton.addEventListener('click', sendMessage);
    
    // Panel toggle handlers
    elements.collapseButton.addEventListener('click', () => {
        elements.actorPanel.classList.add('collapsed');
        elements.togglePanelButton.style.display = 'flex';
    });
    
    elements.togglePanelButton.addEventListener('click', () => {
        elements.actorPanel.classList.remove('collapsed');
        elements.togglePanelButton.style.display = 'none';
    });
});

// Cleanup
window.addEventListener('unload', () => {
    if (ws) {
        ws.close();
    }
});
