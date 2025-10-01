function showMessage(area, text, type = 'error') {
    area.textContent = text;
    area.className = `message ${type}`;
}

function addChatMessage(message) {
    const chatMessages = document.getElementById('chat-messages');
    const msgDiv = document.createElement('div');
    msgDiv.classList.add('chat-message');
    
    const timestamp = new Date(message.timestamp).toLocaleTimeString();

    msgDiv.innerHTML = `
        <span class="timestamp">[${timestamp}]</span>
        <span class="username">${message.sender_username}:</span>
        <span class="content">${message.content}</span>
    `;
    chatMessages.appendChild(msgDiv);
    chatMessages.scrollTop = chatMessages.scrollHeight;
}

export { showMessage, addChatMessage };
