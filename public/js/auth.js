import { showMessage } from './ui.js';
import { initWebSocket, sendWsMessage, getWebSocket } from './websocket.js';
import { t } from './i18n.js';

function handleAuth(action, handlers) {
    const usernameInput = document.getElementById('username-input');
    const passwordInput = document.getElementById('password-input');
    const messageArea = document.getElementById('message-area');

    const username = usernameInput.value;
    const password = passwordInput.value;
    if (!username || !password) {
        showMessage(messageArea, t('authEmptyFields'));
        return;
    }
    
    const payload = { username, password };

    const ws = getWebSocket();
    if (ws && ws.readyState === WebSocket.OPEN) {
        sendWsMessage(action, payload);
    } else {
        initWebSocket(() => {
            sendWsMessage(action, payload);
        }, handlers);
    }
}

export { handleAuth };