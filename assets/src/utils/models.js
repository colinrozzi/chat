// Model utility functions
import { 
  messageChain, models, lastUsedModelId, 
  totalCost, totalInputTokens, totalOutputTokens, totalMessages,
  setTotalCost, setTotalInputTokens, setTotalOutputTokens, setTotalMessages
} from '../components/app.js';
import { sortMessageChain } from './message-chain.js';
import { updateStatsDisplay } from './ui.js';

// Find the last used model ID by scanning the message chain
export function findLastUsedModel() {
  // Use the sorted message chain to get messages in chronological order
  const sortedMessages = sortMessageChain();
  
  // Find the last assistant message in the chain
  for (let i = sortedMessages.length - 1; i >= 0; i--) {
    const message = sortedMessages[i];
    if (message.data && message.data.Chat && message.data.Chat.Assistant) {
      const assistantMsg = message.data.Chat.Assistant;
      let model;
      
      // Handle nested structure
      if (assistantMsg.Claude) {
        model = assistantMsg.Claude.model;
      } else if (assistantMsg.Gemini) {
        model = assistantMsg.Gemini.model;
      } else if (assistantMsg.OpenRouter) {
        model = assistantMsg.OpenRouter.model;
      } else {
        // Fallback for older structure
        model = assistantMsg.model;
      }
      
      if (model) {
        console.log(`Found last used model: ${model}`);
        import('../components/app.js').then(({ setLastUsedModelId }) => {
          setLastUsedModelId(model);
        });
        return model;
      }
    }
  }
  
  // If no assistant message found, return null
  console.log('No assistant message found in the chain, no lastUsedModelId set');
  return null;
}

// Helper to get max tokens for a model based on model data
export function getModelMaxTokens(modelId) {
  // Find the model in our models array
  const model = models.find(m => m.id === modelId);
  if (model && model.max_tokens) {
    return model.max_tokens;
  }
  
  // Fallback values if not in models array
  
  // Check for OpenRouter models
  if (modelId?.includes('/')) {
    // Check specifically for Llama 4 Maverick free
    if (modelId === "meta-llama/llama-4-maverick:free" || 
        modelId === "llama-4-maverick:free" || 
        modelId === "llama-4-maverick-free") {
      return 1000000; // 1 million tokens context window
    }
  }
  
  switch(modelId) {
    // Gemini models
    case "gemini-2.0-flash":
    case "gemini-2.0-pro": return 32768;
        
    // Claude 3.7 models
    case "claude-3-7-sonnet-20250219": return 8192;
    
    // Claude 3.5 models
    case "claude-3-5-sonnet-20241022":
    case "claude-3-5-haiku-20241022":
    case "claude-3-5-sonnet-20240620": return 8192;
    
    // Claude 3 models
    case "claude-3-opus-20240229":
    case "claude-3-sonnet-20240229":
    case "claude-3-haiku-20240307": return 4096;
    
    // Claude 2 models
    case "claude-2.1":
    case "claude-2.0": return 4096;
    
    // Default case
    default: return 4096; // Conservative default
  }
}

// Get pricing for a specific model
export function getModelPricing(modelId) {
  // Check if it's a Gemini model
  if (modelId?.startsWith("gemini-")) {
    if (modelId === "gemini-2.0-flash") {
      return { inputCost: 0.35, outputCost: 1.05 };
    } else if (modelId === "gemini-2.0-pro") {
      return { inputCost: 3.50, outputCost: 10.50 };
    }
  }
  
  // Check for OpenRouter models
  if (modelId?.includes('/')) {
    // Check specifically for Llama 4 Maverick free
    if (modelId === "meta-llama/llama-4-maverick:free" || 
        modelId === "llama-4-maverick:free" || 
        modelId === "llama-4-maverick-free") {
      return { inputCost: 0.00, outputCost: 0.00 }; // Free model
    }
    
    // For other OpenRouter models, provide default pricing or unknown
    // This is a placeholder - real pricing would depend on the specific model
    return { inputCost: null, outputCost: null };
  }
  
  // Claude model pricing
  switch(modelId) {
    // Claude 3.7 models
    case "claude-3-7-sonnet-20250219":
      return { inputCost: 3.00, outputCost: 15.00 };
        
    // Claude 3.5 models
    case "claude-3-5-sonnet-20241022":
    case "claude-3-5-sonnet-20240620":
      return { inputCost: 3.00, outputCost: 15.00 };
        
    case "claude-3-5-haiku-20241022":
      return { inputCost: 0.80, outputCost: 4.00 };
        
    // Claude 3 models
    case "claude-3-opus-20240229":
      return { inputCost: 15.00, outputCost: 75.00 };
        
    case "claude-3-sonnet-20240229":
      return { inputCost: 3.00, outputCost: 15.00 };
        
    case "claude-3-haiku-20240307":
      return { inputCost: 0.25, outputCost: 1.25 };
        
    // Default or unknown model - use null to indicate unknown pricing
    default:
      return { inputCost: null, outputCost: null };
  }
}

// Calculate the cost of a message
export function calculateMessageCost(usage, addToTotal = false, modelId = null) {
  // If no model ID provided, use the last used model ID
  modelId = modelId || lastUsedModelId || "claude-3-7-sonnet-20250219";
  
  // Get pricing for this specific model
  const pricing = getModelPricing(modelId);
  
  // If pricing is unknown for this model, return "Unknown"
  if (pricing.inputCost === null || pricing.outputCost === null) {
    return "Unknown";
  }
  
  // Handle different field names for gemini vs claude
  const inputTokens = usage.input_tokens || usage.prompt_tokens || 0;
  const outputTokens = usage.output_tokens || usage.completion_tokens || 0;
  
  const inputCost = (inputTokens / 1000000) * pricing.inputCost;
  const outputCost = (outputTokens / 1000000) * pricing.outputCost;
  const messageCost = inputCost + outputCost;
  
  // Update total cost only when explicitly requested
  if (addToTotal) {
    setTotalCost(totalCost + messageCost);
    setTotalInputTokens(totalInputTokens + inputTokens);
    setTotalOutputTokens(totalOutputTokens + outputTokens);
    setTotalMessages(totalMessages + 1);
    updateStatsDisplay();
  }
  
  // Format to 4 decimal places
  return messageCost.toFixed(4);
}
