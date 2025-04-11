// Model selector component for selecting AI models
import { models, lastUsedModelId } from '../app.js';
import { elements } from '../utils/elements.js';
import { getModelMaxTokens, getModelPricing } from '../utils/models.js';

// Populate the model selector dropdown
export function populateModelSelector() {
  if (!elements.controlsModelSelector || !models || models.length === 0) return;
  
  // Save the currently selected model if any
  const currentSelection = elements.controlsModelSelector.value;
  
  // Group models by provider
  const claudeModels = models.filter(m => !m.provider || m.provider === 'claude');
  const geminiModels = models.filter(m => m.provider === 'gemini');
  const openrouterModels = models.filter(m => m.provider === 'openrouter');
  
  // Sort Claude models with the most recent first
  const sortedClaudeModels = [...claudeModels].sort((a, b) => {
    // Special case: always put 3.7 Sonnet at the top
    if (a.id === 'claude-3-7-sonnet-20250219') return -1;
    if (b.id === 'claude-3-7-sonnet-20250219') return 1;
    return b.id.localeCompare(a.id);
  });
  
  // Clear current options
  elements.controlsModelSelector.innerHTML = '';
  
  // Create Claude group
  const claudeGroup = document.createElement('optgroup');
  claudeGroup.label = 'Claude Models';
  
  // Add Claude options
  sortedClaudeModels.forEach(model => {
    const option = document.createElement('option');
    option.value = model.id;
    option.textContent = model.display_name;
    claudeGroup.appendChild(option);
  });
  
  // Create Gemini group
  const geminiGroup = document.createElement('optgroup');
  geminiGroup.label = 'Gemini Models';
  
  // Add Gemini options
  geminiModels.forEach(model => {
    const option = document.createElement('option');
    option.value = model.id;
    option.textContent = model.display_name;
    geminiGroup.appendChild(option);
  });
  
  // Create OpenRouter group
  const openrouterGroup = document.createElement('optgroup');
  openrouterGroup.label = 'OpenRouter Models';
  
  // Add OpenRouter options
  openrouterModels.forEach(model => {
    const option = document.createElement('option');
    option.value = model.id;
    option.textContent = model.display_name;
    openrouterGroup.appendChild(option);
  });
  
  // Add groups to selector
  elements.controlsModelSelector.appendChild(claudeGroup);
  elements.controlsModelSelector.appendChild(geminiGroup);
  elements.controlsModelSelector.appendChild(openrouterGroup);
  
  // Prioritize using the last used model if available
  if (lastUsedModelId && models.some(m => m.id === lastUsedModelId)) {
    elements.controlsModelSelector.value = lastUsedModelId;
    console.log(`Set model selector to last used model: ${lastUsedModelId}`);
  } 
  // Otherwise restore previous selection if possible
  else if (currentSelection && models.some(m => m.id === currentSelection)) {
    elements.controlsModelSelector.value = currentSelection;
    console.log(`Restored previous selection: ${currentSelection}`);
  } 
  // Default to the first option if nothing else works
  else if (models.length > 0) {
    elements.controlsModelSelector.value = models[0].id;
    console.log(`Defaulted to first model: ${models[0].id}`);
  }
  
  // Update model context window info
  updateModelInfo();
}

// Update the selected model in the UI
export function updateModelSelector() {
  // Check if we have a model to select and the model selector exists
  if (!lastUsedModelId || !elements.controlsModelSelector) return;
  
  // Check if the model is available in the models list
  if (models.some(model => model.id === lastUsedModelId)) {
    console.log(`Setting model selector to last used model: ${lastUsedModelId}`);
    elements.controlsModelSelector.value = lastUsedModelId;
    // Update model info display
    updateModelInfo();
  } else {
    console.log(`Last used model ${lastUsedModelId} not found in available models`);
  }
}

// Update the model info display in the controls sidebar
export function updateModelInfo() {
  if (!elements.controlsModelSelector || !elements.modelContextWindow) return;
  
  const selectedModelId = elements.controlsModelSelector.value;
  const maxTokens = getModelMaxTokens(selectedModelId);
  const modelInfo = document.getElementById('modelInfo');
  
  // Format with commas
  elements.modelContextWindow.textContent = new Intl.NumberFormat().format(maxTokens) + ' tokens';
  
  // Update cost information based on the model
  const costInfoElem = modelInfo.querySelector('.info-value:not(#modelContextWindow)');
  if (costInfoElem) {
    // Get model pricing
    const pricing = getModelPricing(selectedModelId);
    
    // Update cost display
    if (pricing.inputCost === null || pricing.outputCost === null) {
      costInfoElem.textContent = 'Cost information unavailable';
    } else if (pricing.inputCost === 0 && pricing.outputCost === 0) {
      costInfoElem.textContent = 'Free';
    } else {
      costInfoElem.textContent = '$' +
       + pricing.inputCost.toFixed(2) + ' / '
       + pricing.outputCost.toFixed(2) + ' per 1M tokens (in/out)';
    }
  }
}
