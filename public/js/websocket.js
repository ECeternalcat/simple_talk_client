import { playAudioChunk } from './audio.js';

let ws;
let currentUser;

function getWebSocket() {
    return ws;
}

function initWebSocket(onOpenCallback, handlers) {
    // Always create a new WebSocket connection for each client instance.
    // The previous logic prevented this, causing issues with multiple tabs.

    const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
    ws = new WebSocket(`${protocol}://${window.location.host}/ws`);

    ws.onopen = () => {
        console.log('[WS] Connection opened.');
        if (onOpenCallback) {
            onOpenCallback();
        }
    };

    ws.onmessage = async (event) => {
        if (event.data instanceof Blob || event.data instanceof ArrayBuffer) {
            // It's binary data (audio)
            const audioData = new Float32Array(await event.data.arrayBuffer());
            playAudioChunk(audioData);
            return;
        }

        // It's text data (JSON)
        const msg = JSON.parse(event.data);
        console.log('[WS] Received message:', msg);

        if (handlers[msg.type]) {
            handlers[msg.type](msg.payload);
        }
    };

    ws.onerror = (e) => {
        console.error('[WS] WebSocket error:', e);
        if (handlers.error) {
            handlers.error(e);
        }
    };
    ws.onclose = (e) => {
        console.log(`[WS] WebSocket disconnected. Code: ${e.code}, Reason: ${e.reason}`);
        if (handlers.close) {
            handlers.close(e);
        }
        ws = null;
    };
}

function sendWsMessage(type, payload = {}) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        console.log(`[WS] Sending message: { type: '${type}', ... }`);
        ws.send(JSON.stringify({ type, payload }));
    } else {
        console.error(`[WS] Could not send message. WebSocket is not open. State: ${ws ? ws.readyState : 'null'}`);
    }
}

function setCurrentUser(user) {
    currentUser = user;
}

export { initWebSocket, sendWsMessage, currentUser, setCurrentUser, getWebSocket };