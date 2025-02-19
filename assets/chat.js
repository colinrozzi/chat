// State management
let messageChain = [];
let currentHead = null;
let ws = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;

// WebSocket setup
function connectWebSocket() {
    console.log("Connecting to WebSocket...");
    updateConnectionStatus('connecting');
    
    ws = new WebSocket(`ws://localhost:{{WEBSOCKET_PORT}}/`);
    
    ws.onopen = () => {
        console.log("WebSocket connected");
        updateConnectionStatus('connected');
        reconnectAttempts = 0;
        console.log("Requesting initial head");
        sendWebSocketMessage({ type: 'get_head' });
    };
    
    ws.onclose = () => {
        console.log("WebSocket disconnected");
        updateConnectionStatus('disconnected');
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttempts++;
            setTimeout(connectWebSocket, 1000 * Math.min(reconnectAttempts, 30));
        }
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

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

function requestMessage(messageId) {
    console.log("Requesting message:", messageId);
    sendWebSocketMessage({
        type: 'get_message',
        message_id: messageId
    });
}

function sendWebSocketMessage(message) {
    console.log("Sending WebSocket message:", message);
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(message));
    } else {
        console.error("WebSocket not connected, can't send message");
    }
}

function updateConnectionStatus(status) {
    const statusElement = document.getElementById('connectionStatus');
    if (statusElement) {
        statusElement.className = 'connection-status ' + status;
        statusElement.textContent = status.charAt(0).toUpperCase() + status.slice(1);
    }
}

function handleWebSocketMessage(data) {
    console.log("Processing message:", data);

    if (data.type === 'messages_updated') {
        console.log("Messages updated, head:", data.head);
        if (data.head) {
            currentHead = data.head;
            document.querySelector('.head-id').textContent = `Head: ${data.head}`;
            requestMessage(data.head); // Request the head message
        }
        return;
    }

    if (data.type === 'head') {
        console.log("Received head:", data.head);
        if (data.head) {
            currentHead = data.head;
            document.querySelector('.head-id').textContent = `Head: ${data.head}`;
            requestMessage(data.head); // Request the head message
        }
        return;
    }

    if (data.type === 'message' && data.message) {
        console.log("Received message:", data.message);
        const message = data.message;
        
        // Add to message chain if not already present
        if (!messageChain.find(m => m.id === message.id)) {
            messageChain.push(message);
        }

        // If this message has a parent and we don't have it yet, request it
        if (message.parent && !messageChain.find(m => m.id === message.parent)) {
            requestMessage(message.parent);
        }

        renderMessages();
    }
}

// Message rendering
function renderMessages() {
    console.log("Rendering messages, chain length:", messageChain.length);
    const messageArea = document.getElementById('messageArea');
    if (!messageArea) return;

    if (messageChain.length === 0) {
        messageArea.innerHTML = '<div class="empty-state">No messages yet.<br>Start the conversation!</div>';
        return;
    }

    // Sort messages by parent relationship
    const sortedChain = sortMessageChain();
    console.log("Sorted message chain:", sortedChain);
    
    // Render messages
    messageArea.innerHTML = sortedChain.map(entry => {
        if (entry.data.Chat) {
            return renderChatMessage(entry.data.Chat);
        } else if (entry.data.Child) {
            return renderChildMessage(entry.data.Child);
        }
        return '';
    }).join('');

    // Scroll to bottom
    messageArea.scrollTop = messageArea.scrollHeight;
}

function sortMessageChain() {
    const sorted = [];
    const visited = new Set();

    function addMessage(id) {
        if (!id || visited.has(id)) return;
        
        const message = messageChain.find(m => m.id === id);
        if (!message) return;

        // First add parent if exists
        if (message.parent) {
            addMessage(message.parent);
        }

        // Then add this message
        if (!visited.has(id)) {
            sorted.push(message);
            visited.add(id);
        }
    }

    // Start from head
    if (currentHead) {
        addMessage(currentHead);
    } else {
        // If no head, add all messages in order
        [...messageChain].sort((a, b) => {
            if (!a.parent && b.parent) return -1;
            if (a.parent && !b.parent) return 1;
            return 0;
        }).forEach(message => {
            addMessage(message.id);
        });
    }

    return sorted;
}

function renderChatMessage(message) {
    console.log("Rendering chat message:", message);
    const role = message.role;
    const content = formatMessageContent(message.content);
    
    return `
        <div class="message ${role}">
            ${content}
        </div>
    `;
}

function renderChildMessage(childMessage) {
    console.log("Rendering child message:", childMessage);
    return `
        <div class="child-response">
            <div class="actor-response-content">
                <div class="actor-response-header">
                    <span class="actor-name">Actor: ${childMessage.child_id}</span>
                </div>
                ${formatMessageContent(childMessage.text)}
            </div>
        </div>
    `;
}

function formatMessageContent(content) {
    if (!content) return '';
    
    // Escape HTML
    let text = content
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;")
        .replace(/\n/g, '<br>');
    
    // Format code blocks
    text = text.replace(/```([^`]+)```/g, (match, code) => 
        `<pre><code>${code}</code></pre>`
    );
    
    // Format inline code
    text = text.replace(/`([^`]+)`/g, (match, code) => 
        `<code>${code}</code>`
    );
    
    return text;
}

// Message sending
function sendMessage() {
    const messageInput = document.getElementById('messageInput');
    const content = messageInput.value.trim();
    
    if (!content || !ws || ws.readyState !== WebSocket.OPEN) {
        console.log("Cannot send message:", {
            content: !!content,
            wsExists: !!ws,
            wsState: ws ? ws.readyState : 'no websocket'
        });
        return;
    }
    
    console.log("Sending message:", content);
    // Send message
    sendWebSocketMessage({
        type: 'send_message',
        content: content
    });
    
    // Clear input
    messageInput.value = '';
    messageInput.style.height = '2.5rem';
    messageInput.focus();
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    console.log("Initializing chat application");
    // Initialize WebSocket
    connectWebSocket();
    
    // Setup message input
    const messageInput = document.getElementById('messageInput');
    const sendButton = document.getElementById('sendButton');
    
    // Auto-resize textarea
    messageInput.addEventListener('input', () => {
        messageInput.style.height = '2.5rem';
        messageInput.style.height = Math.min(messageInput.scrollHeight, 200) + 'px';
    });
    
    // Send message on Enter (not Shift+Enter)
    messageInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
    
    // Send button click
    sendButton.addEventListener('click', sendMessage);
    
    // Global shortcut to focus input
    document.addEventListener('keydown', (event) => {
        if (event.key === '/' && document.activeElement !== messageInput) {
            event.preventDefault();
            messageInput.focus();
        }
    });
});

// Handle visibility changes
document.addEventListener('visibilitychange', () => {
    if (!document.hidden && (!ws || ws.readyState !== WebSocket.OPEN)) {
        connectWebSocket();
    }
});

// Cleanup
window.addEventListener('unload', () => {
    if (ws) {
        ws.close();
    }
});
