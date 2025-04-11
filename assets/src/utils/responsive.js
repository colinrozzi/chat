// Responsive design utilities
import { elements } from './elements.js';

// Check if mobile view and collapse sidebar if needed
export function checkMobileView() {
  const isMobile = window.innerWidth <= 768;
  
  if (isMobile) {
    // Collapse both sidebars on mobile
    if (elements.chatSidebar) {
      elements.chatSidebar.classList.add('collapsed');
      if (elements.expandChatSidebarButton) {
        elements.expandChatSidebarButton.classList.add('visible');
      }
    }
    
    if (elements.chatControlsSidebar) {
      elements.chatControlsSidebar.classList.add('collapsed');
      if (elements.expandChatControlsButton) {
        elements.expandChatControlsButton.classList.add('visible');
      }
    }
  } else if (window.innerWidth <= 1200) {
    // On tablets, only collapse the controls sidebar
    if (elements.chatControlsSidebar) {
      elements.chatControlsSidebar.classList.add('collapsed');
      if (elements.expandChatControlsButton) {
        elements.expandChatControlsButton.classList.add('visible');
      }
    }
  }
}
