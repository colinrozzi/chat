// Message component for handling individual messages
import { messageChain, totalCost, totalInputTokens, totalOutputTokens, totalMessages } from '../app.js';
import { elements } from '../utils/elements.js';
import { requestMessage } from '../services/websocket.js';
import { renderMessages } from './chat.js';
import { updateStatsDisplay } from '../utils/ui.js';
import { scrollToBottom } from '../utils/ui.js';
import { removeTypingIndicator } from '../utils/typing-indicator.js';

// Handle a new message from the server
export function handleNewMessage(message, wsConnection) {
  console.log('Handling new message:', message);
  console.log('Message chain before handling:', {
    length: messageChain.length,
    ids: messageChain.map(m => m.id),
    currentHead: window.currentHead
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
      const assistantMsg = message.data.Chat.Assistant;
      
      // Handle the new nested structure (Claude, Gemini, or OpenRouter)
      if (assistantMsg.Claude) {
        const claude = assistantMsg.Claude;
        // Calculate cost based on model-specific pricing if available
        if (claude.input_cost_per_million_tokens !== undefined && 
            claude.output_cost_per_million_tokens !== undefined) {
          
          if (claude.input_cost_per_million_tokens !== null && 
              claude.output_cost_per_million_tokens !== null) {
              
              const inputCost = (claude.usage.input_tokens / 1000000) * claude.input_cost_per_million_tokens;
              const outputCost = (claude.usage.output_tokens / 1000000) * claude.output_cost_per_million_tokens;
              totalCost += (inputCost + outputCost);
              totalInputTokens += claude.usage.input_tokens;
              totalOutputTokens += claude.usage.output_tokens;
              totalMessages++;
              updateStatsDisplay();
          }
        } else {
          // Fallback to calculateMessageCost function
          import('../utils/models.js').then(({ calculateMessageCost }) => {
            calculateMessageCost(claude.usage, true, claude.model);
          });
        }
        
        // Store the model ID from the last assistant message
        window.lastUsedModelId = claude.model;
        console.log(`Stored last used model ID: ${window.lastUsedModelId}`);
      } else if (assistantMsg.Gemini) {
        const gemini = assistantMsg.Gemini;
        // Calculate cost based on model-specific pricing if available
        if (gemini.input_cost_per_million_tokens !== undefined && 
            gemini.output_cost_per_million_tokens !== undefined) {
              
            if (gemini.input_cost_per_million_tokens !== null && 
                gemini.output_cost_per_million_tokens !== null) {
                
                const inputCost = (gemini.usage.prompt_tokens / 1000000) * gemini.input_cost_per_million_tokens;
                const outputCost = (gemini.usage.completion_tokens / 1000000) * gemini.output_cost_per_million_tokens;
                totalCost += (inputCost + outputCost);
                totalInputTokens += gemini.usage.prompt_tokens;
                totalOutputTokens += gemini.usage.completion_tokens;
                totalMessages++;
                updateStatsDisplay();
            }
        } else {
          // Fallback to calculateMessageCost function
          import('../utils/models.js').then(({ calculateMessageCost }) => {
            calculateMessageCost(gemini.usage, true, gemini.model);
          });
        }
        
        // Store the model ID from the last assistant message
        window.lastUsedModelId = gemini.model;
        console.log(`Stored last used model ID: ${window.lastUsedModelId}`);
      } else if (assistantMsg.OpenRouter) {
        const openrouter = assistantMsg.OpenRouter;
        // Calculate cost based on model-specific pricing if available
        if (openrouter.input_cost_per_million_tokens !== undefined && 
            openrouter.output_cost_per_million_tokens !== undefined) {
              
            if (openrouter.input_cost_per_million_tokens !== null && 
                openrouter.output_cost_per_million_tokens !== null) {
                
                const inputCost = (openrouter.usage.prompt_tokens / 1000000) * openrouter.input_cost_per_million_tokens;
                const outputCost = (openrouter.usage.completion_tokens / 1000000) * openrouter.output_cost_per_million_tokens;
                totalCost += (inputCost + outputCost);
                totalInputTokens += openrouter.usage.prompt_tokens;
                totalOutputTokens += openrouter.usage.completion_tokens;
                totalMessages++;
                updateStatsDisplay();
            } else if (openrouter.usage.cost !== null) {
                totalCost += openrouter.usage.cost;
                totalInputTokens += openrouter.usage.prompt_tokens || 0;
                totalOutputTokens += openrouter.usage.completion_tokens || 0;
                totalMessages++;
                updateStatsDisplay();
            }
        } else {
          // Fallback to calculateMessageCost function
          import('../utils/models.js').then(({ calculateMessageCost }) => {
            calculateMessageCost(openrouter.usage, true, openrouter.model);
          });
        }
        
        // Store the model ID from the last assistant message
        window.lastUsedModelId = openrouter.model;
        console.log(`Stored last used model ID: ${window.lastUsedModelId}`);
      }
      
      // Update the selected model in the UI if it exists
      import('./model-selector.js').then(({ updateModelSelector }) => {
        updateModelSelector();
      });
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
        requestMessage(parentId, wsConnection);
      } else {
        console.log(`Parent already in chain: ${parentId}`);
      }
    }
  } else {
    console.log('Message has no parents');
  }

  // Reset waiting state and remove typing indicator
  window.isWaitingForResponse = false;
  removeTypingIndicator();
  elements.sendButton.disabled = !elements.messageInput.value.trim();
  
  // Enable generate button if we have messages
  elements.generateButton.disabled = (messageChain.length === 0);

  renderMessages();
  scrollToBottom();
}
