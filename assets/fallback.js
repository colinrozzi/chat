// Fallback script that loads if the main script fails
(function() {
    // Check if the main app initialized successfully
    if (!window.appInitialized) {
        console.log('Main script failed to initialize. Loading fallback...');
        
        // Show a helpful error message in the UI
        function showErrorUI() {
            // Find the main container
            const messagesContainer = document.getElementById('messagesContainer');
            if (messagesContainer) {
                messagesContainer.innerHTML = `
                    <div style="padding: 2rem; text-align: center;">
                        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#EF4444" stroke-width="2">
                            <circle cx="12" cy="12" r="10"></circle>
                            <line x1="12" y1="8" x2="12" y2="12"></line>
                            <line x1="12" y1="16" x2="12.01" y2="16"></line>
                        </svg>
                        <h2 style="color: #F3F4F6; margin-top: 1rem;">Connection Error</h2>
                        <p style="color: #9CA3AF;">Failed to connect to the chat server. Please check the following:</p>
                        <ul style="color: #9CA3AF; text-align: left; max-width: 500px; margin: 1rem auto; line-height: 1.6;">
                            <li>Make sure the chat server is running</li>
                            <li>Check if the port number is correct (current URL: ${window.location.href})</li>
                            <li>Try refreshing the page</li>
                            <li>Check the browser console for error messages</li>
                        </ul>
                        <div style="margin-top: 1.5rem;">
                            <a href="debug.html" style="display: inline-block; background: #6366F1; color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; text-decoration: none; margin-right: 1rem;">
                                Run Diagnostics
                            </a>
                            <button onclick="location.reload()" style="background: transparent; border: 1px solid #6366F1; color: #6366F1; padding: 0.75rem 1.5rem; border-radius: 0.5rem; cursor: pointer;">
                                Reload Page
                            </button>
                        </div>
                    </div>
                `;
            }
            
            // Update connection status indicator
            const connectionStatus = document.getElementById('connectionStatus');
            if (connectionStatus) {
                connectionStatus.innerHTML = `
                    <div class="status-indicator"></div>
                    <span>Connection Failed</span>
                `;
                connectionStatus.className = 'connection-status disconnected';
            }
            
            // Disable input controls
            const sendButton = document.getElementById('sendButton');
            if (sendButton) sendButton.disabled = true;
            
            const generateButton = document.getElementById('generateButton');
            if (generateButton) generateButton.disabled = true;
            
            const messageInput = document.getElementById('messageInput');
            if (messageInput) {
                messageInput.disabled = true;
                messageInput.placeholder = 'Connection to server failed...';
            }
        }
        
        // Try to diagnose the WebSocket connection issue
        function checkWebSocketConnection() {
            console.log('Attempting diagnostic WebSocket connection...');
            
            // Get port from current URL or use default
            let port = window.location.port || '8084';
            
            // Try to connect to the WebSocket
            try {
                const ws = new WebSocket(`ws://localhost:${port}/ws`);
                
                ws.onopen = function() {
                    console.log('Diagnostic WebSocket connected successfully!');
                    // Could add UI to show this success and offer to refresh
                };
                
                ws.onerror = function(error) {
                    console.error('Diagnostic WebSocket error:', error);
                    // Just log the error, UI already shows the problem
                };
                
                // Set timeout to close the connection attempt after 5 seconds
                setTimeout(function() {
                    if (ws.readyState !== WebSocket.CLOSED && ws.readyState !== WebSocket.CLOSING) {
                        ws.close();
                    }
                }, 5000);
            } catch (error) {
                console.error('Failed to create diagnostic WebSocket:', error);
            }
        }
        
        // Initialize fallback
        function init() {
            showErrorUI();
            checkWebSocketConnection();
            
            // Add event listeners for the sidebar toggles to ensure they work
            const collapseChatSidebarButton = document.getElementById('collapseChatSidebarButton');
            const expandChatSidebarButton = document.getElementById('expandChatSidebarButton');
            const chatSidebar = document.getElementById('chatSidebar');
            
            if (collapseChatSidebarButton && chatSidebar) {
                collapseChatSidebarButton.addEventListener('click', function() {
                    chatSidebar.classList.add('collapsed');
                    if (expandChatSidebarButton) expandChatSidebarButton.classList.add('visible');
                });
            }
            
            if (expandChatSidebarButton && chatSidebar) {
                expandChatSidebarButton.addEventListener('click', function() {
                    chatSidebar.classList.remove('collapsed');
                    expandChatSidebarButton.classList.remove('visible');
                });
            }
            
            const collapseChatControlsButton = document.getElementById('collapseChatControlsButton');
            const expandChatControlsButton = document.getElementById('expandChatControlsButton');
            const chatControlsSidebar = document.getElementById('chatControlsSidebar');
            
            if (collapseChatControlsButton && chatControlsSidebar) {
                collapseChatControlsButton.addEventListener('click', function() {
                    chatControlsSidebar.classList.add('collapsed');
                    if (expandChatControlsButton) expandChatControlsButton.classList.add('visible');
                });
            }
            
            if (expandChatControlsButton && chatControlsSidebar) {
                expandChatControlsButton.addEventListener('click', function() {
                    chatControlsSidebar.classList.remove('collapsed');
                    expandChatControlsButton.classList.remove('visible');
                });
            }
        }
        
        // Run the initialization
        init();
    }
})();
