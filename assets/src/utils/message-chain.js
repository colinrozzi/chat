// Message chain utilities
import { messageChain, currentHead } from '../components/app.js';

// Sort the message chain to get messages in chronological order
export function sortMessageChain() {
  console.log('Sorting message chain:', {
    chainLength: messageChain.length,
    currentHead: currentHead
  });
  
  // Create a map for fast lookups
  const messagesById = {};
  messageChain.forEach(msg => {
    messagesById[msg.id] = msg;
  });
  
  // Track visited messages to handle potential cycles
  const visited = new Set();
  const result = [];
  const missingParents = new Set();
  
  // For topological sort - start with all head nodes (nodes with no parents)
  // In our DAG traversal, we'll use a recursive DFS approach
  function processMessage(message, level = 0) {
    console.log(`Processing message: ${message.id} at level ${level}`);
    if (visited.has(message.id)) {
      console.log(`- Already visited ${message.id}, skipping`);
      return;
    }
    visited.add(message.id);
    
    // Process parents first (recursively)
    if (message.parents && message.parents.length > 0) {
      console.log(`- Message ${message.id} has ${message.parents.length} parents`);
      for (const parentId of message.parents) {
        const parent = messagesById[parentId];
        if (parent) {
          console.log(`- Processing parent: ${parentId}`);
          processMessage(parent, level + 1);
        } else {
          console.log(`- MISSING PARENT: ${parentId} for message ${message.id}`);
          missingParents.add(parentId);
        }
      }
    } else {
      console.log(`- Message ${message.id} has no parents`);
    }
    
    // Add this message to the result
    result.push(message);
  }
  
  // Find the head message (latest message)
  if (currentHead && messagesById[currentHead]) {
    console.log(`Starting traversal from head: ${currentHead}`);
    processMessage(messagesById[currentHead]);
  } else {
    console.log('No current head or head not found in message chain');
    // Without a clear head, process all messages
    console.log('Processing all messages as fallback');
    messageChain.forEach(msg => {
      if (!visited.has(msg.id)) {
        processMessage(msg);
      }
    });
  }
  
  if (missingParents.size > 0) {
    console.warn('MISSING PARENTS DETECTED:', Array.from(missingParents));
  }
  
  console.log(`Sorted message chain: ${result.length} messages`);
  return result;
}
