// Main entry point for the chat application
import { initializeApp } from './components/app.js';
import { setupEventListeners } from './components/events.js';
import { checkMobileView } from './utils/responsive.js';

// Initialize the application when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
  // Initialize the chat application
  initializeApp();
  
  // Set up event listeners
  setupEventListeners();
  
  // Check for mobile view on load
  checkMobileView();
  
  // Handle window resize
  window.addEventListener('resize', checkMobileView);
});
