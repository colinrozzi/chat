// Text and content formatting utilities

// Format message content with code highlighting, etc.
export function formatMessageContent(content) {
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

// Format JSON data for display
export function formatJsonData(data) {
  try {
    // Convert the data to a formatted string with 2-space indentation
    return JSON.stringify(data, null, 2)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  } catch (error) {
    console.error('Error formatting JSON data:', error);
    return 'Error displaying data';
  }
}

// Basic HTML sanitization to prevent XSS
export function sanitizeHTML(html) {
  if (!html) return '';

  // Create a temporary element
  const tempDiv = document.createElement('div');
  tempDiv.innerHTML = html;

  // Remove potentially dangerous elements and attributes
  const dangerous = ['script', 'iframe', 'object', 'embed', 'form'];
  dangerous.forEach(tag => {
    const elements = tempDiv.getElementsByTagName(tag);
    while (elements.length > 0) {
      elements[0].parentNode.removeChild(elements[0]);
    }
  });

  // Remove dangerous attributes from all elements
  const allElements = tempDiv.getElementsByTagName('*');
  for (let i = 0; i < allElements.length; i++) {
    const element = allElements[i];
    const attrs = element.attributes;
    for (let j = attrs.length - 1; j >= 0; j--) {
      const attr = attrs[j];
      if (attr.name.startsWith('on') || attr.name === 'href' && attr.value.startsWith('javascript:')) {
        element.removeAttribute(attr.name);
      }
    }
  }

  return tempDiv.innerHTML;
}
