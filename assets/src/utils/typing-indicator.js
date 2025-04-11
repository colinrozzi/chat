// Typing indicator utilities for chat interface
import { elements } from './elements.js';
import { scrollToBottom } from './ui.js';

// Add a typing indicator to show the assistant is thinking
export function addTypingIndicator() {
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

// Remove the typing indicator
export function removeTypingIndicator() {
  const indicator = document.getElementById('typingIndicator');
  if (indicator) {
    indicator.remove();
  }
}
