// WebSocket service for communication with the server
import { ws, reconnectAttempts, MAX_RECONNECT_ATTEMPTS, RECONNECT_DELAY } from '../components/app.js';
import { elements } from '../utils/elements.js';
import { updateConnectionStatus, showError } from '../utils/ui.js';
import { handleWebSocketMessage } from './message-handler.js';

// Initialize the websocket connection
export function connectWebSocket() {
  console.log('Connecting to WebSocket...');
  updateConnectionStatus('connecting');
  
  // Create a new WebSocket connection
  // The {{WEBSOCKET_PORT}} is a placeholder that gets replaced by the server
  const wsConnection = new WebSocket(`ws://localhost:{{WEBSOCKET_PORT}}/ws`);
  
  wsConnection.onopen = () => {
    console.log('WebSocket connected');
    updateConnectionStatus('connected');
    
    // Reset reconnect attempts counter
    reconnectAttempts = 0;
    
    // Request initial state
    sendWebSocketMessage({ type: 'list_chats' }, wsConnection);  // Get available chats
    sendWebSocketMessage({ type: 'get_head' }, wsConnection);    // Initial head query
    sendWebSocketMessage({ type: 'list_models' }, wsConnection); // Get available models
  };
  
  wsConnection.onclose = () => {
    console.log('WebSocket disconnected');
    updateConnectionStatus('disconnected');
    elements.sendButton.disabled = true;
    elements.generateButton.disabled = true;
    
    // Disconnection handling with exponential backoff
    if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
      reconnectAttempts++;
      setTimeout(connectWebSocket, RECONNECT_DELAY * Math.min(reconnectAttempts, 30));
    }
  };
  
  wsConnection.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      console.log('Received WebSocket message:', data);
      
      // Enhanced logging for debugging child actor issues
      if (data.type === 'messages_updated' || data.type === 'head') {
        console.log('HEAD UPDATE - Before processing:', {
          oldHead: global.currentHead,
          newHead: data.head,
          messageChainLength: global.messageChain.length,
          messageIDs: global.messageChain.map(m => m.id)
        });
      } else if (data.type === 'message') {
        console.log('MESSAGE RECEIVED - Details:', {
          messageId: data.message?.id,
          messageParents: data.message?.parents,
          messageType: data.message?.data ? Object.keys(data.message.data)[0] : 'unknown',
          currentChainLength: global.messageChain.length
        });
      }
      
      // Handle the message
      handleWebSocketMessage(data, wsConnection);
      
    } catch (error) {
      console.error('WebSocket message processing error:', error);
      console.error('Raw message:', event.data);
      showError('Failed to process server message');
    }
  };

  wsConnection.onerror = (error) => {
    console.error('WebSocket error:', error);
    showError('Connection error occurred');
  };
  
  // Store the WebSocket connection
  return wsConnection;
}

// Send a message through the WebSocket connection
export function sendWebSocketMessage(message, wsConnection) {
  if (wsConnection?.readyState === WebSocket.OPEN) {
    const messageStr = JSON.stringify(message);
    console.log('Sending WebSocket message:', message);
    wsConnection.send(messageStr);
  } else {
    console.error('Cannot send message: WebSocket not connected', message);
    showError('Not connected to server');
  }
}

// Request a specific message from the server
export function requestMessage(messageId, wsConnection) {
  sendWebSocketMessage({
    type: 'get_message',
    message_id: messageId
  }, wsConnection);
}
