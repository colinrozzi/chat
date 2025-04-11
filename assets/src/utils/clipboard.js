// Clipboard utilities for copying text
import { messageChain } from '../components/app.js';
import { showCopySuccess, showError } from './ui.js';

// Copy message text to clipboard
export function copyMessageText(messageId) {
  // Find the message in the message chain
  const message = messageChain.find(m => m.id === messageId);
  if (!message) return;
  
  // Extract the content based on the message type
  let content = '';
  if (message.data.Chat) {
    if (message.data.Chat.User) {
      content = message.data.Chat.User.content;
    } else if (message.data.Chat.Assistant) {
      const assistantMsg = message.data.Chat.Assistant;
      // Handle nested structure
      if (assistantMsg.Claude) {
        content = assistantMsg.Claude.content;
      } else if (assistantMsg.Gemini) {
        content = assistantMsg.Gemini.content;
      } else if (assistantMsg.OpenRouter) {
        content = assistantMsg.OpenRouter.content;
      } else {
        // Fallback for older structure
        content = assistantMsg.content || '';
      }
    }
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

// Copy message ID to clipboard
export function copyMessageId(messageId) {
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
