import { showMessage } from './ui.js';
import { initWebSocket, sendWsMessage } from './websocket.js';

function handleAuth(action, handlers) {
    const usernameInput = document.getElementById('username-input');
    const passwordInput = document.getElementById('password-input');
    const messageArea = document.getElementById('message-area');

    const username = usernameInput.value;
    const password = passwordInput.value;
    if (!username || !password) {
        showMessage(messageArea, 'Username and password cannot be empty.');
        return;
    }
    
    const payload = { username, password };

    initWebSocket(() => {
        sendWsMessage(action, payload);
    }, handlers);
}

export { handleAuth };
