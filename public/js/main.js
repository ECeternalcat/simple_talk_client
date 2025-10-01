import { handleAuth } from './auth.js';
import { initWebSocket, sendWsMessage, setCurrentUser, currentUser, getWebSocket } from './websocket.js';
import { showMessage, addChatMessage } from './ui.js';
import { renderChatList } from './chats.js';
import { renderFriendRequestList, addFriendRequestToList, renderFriendList } from './friends.js';
import { renderUserList, renderRoomList } from './admin.js';
import { startAudioCapture, stopAudioCapture, setMute } from './audio.js';

document.addEventListener('DOMContentLoaded', () => {
    // --- Views ---
    const setupView = document.getElementById('setup-view');
    const mainView = document.getElementById('main-view');
    const callView = document.getElementById('call-view');
    const adminPanelView = document.getElementById('admin-panel-view');

    // --- Buttons and Inputs ---
    const loginBtn = document.getElementById('login-btn');
    const registerBtn = document.getElementById('register-btn');
    const addFriendBtn = document.getElementById('add-friend-btn');
    const logoutBtn = document.getElementById('logout-btn');
    const backToChatsBtn = document.getElementById('back-to-chats-btn');
    const sendChatBtn = document.getElementById('send-chat-btn');
    const chatInput = document.getElementById('chat-input');
    const chatList = document.getElementById('chat-list');
    const friendList = document.getElementById('friend-list');
    const adminPanelBtn = document.getElementById('admin-panel-btn');
    const closeAdminPanelBtn = document.getElementById('close-admin-panel-btn');
    const startVoiceBtn = document.getElementById('start-voice-btn');
    const muteMicBtn = document.getElementById('mute-mic-btn');
    const voiceControls = document.getElementById('voice-controls');
    const refreshUsersBtn = document.getElementById('refresh-users-btn');
    const refreshRoomsBtn = document.getElementById('refresh-rooms-btn');
    const newUsernameInput = document.getElementById('new-username-input');
    const newPasswordInput = document.getElementById('new-password-input');
    const newUserRoleSelect = document.getElementById('new-user-role-select');
    const createUserBtn = document.getElementById('create-user-btn');
    const shutdownServerBtn = document.getElementById('shutdown-server-btn');
    const userListContainer = document.getElementById('user-list-container');
    const roomListContainer = document.getElementById('room-list-container');

    // --- App State ---
    let isMuted = false;
    let isVoiceActive = false;

    // --- WebSocket Message Handlers ---
    const handlers = {
        // Auth
        register_ok: (payload) => showMessage(document.getElementById('message-area'), payload, 'success'),
        register_fail: (payload) => showMessage(document.getElementById('message-area'), payload, 'error'),
        auth_fail: (payload) => {
            localStorage.removeItem('authToken');
            showMessage(document.getElementById('message-area'), payload, 'error');
            mainView.classList.add('hidden');
            callView.classList.add('hidden');
            setupView.classList.remove('hidden');
        },
        auth_ok: (payload) => {
            if (payload.token) {
                localStorage.setItem('authToken', payload.token);
            }
            setCurrentUser(payload);
            setupView.classList.add('hidden');
            mainView.classList.remove('hidden');

            // Show admin button if user is an admin
            if (payload.role === 'admin') {
                adminPanelBtn.classList.remove('hidden');
            }
        },

        // Room & Chat
        join_ok: (payload) => {
            mainView.classList.add('hidden');
            adminPanelView.classList.add('hidden');
            callView.classList.remove('hidden');
            document.getElementById('status-text').textContent = `Room ID: ${payload.roomId}`;
            
            voiceControls.classList.add('hidden');
            startVoiceBtn.classList.remove('hidden');
            isVoiceActive = false;
            stopAudioCapture();

            const currentRoomId = payload.roomId;
            chatInput.dataset.currentRoomId = currentRoomId;
            document.getElementById('send-chat-btn').onclick = () => {
                 const content = chatInput.value;
                 if (content) {
                    sendWsMessage('send_chat_message', { roomId: currentRoomId, content });
                    chatInput.value = '';
                 }
            };
             chatInput.onkeyup = (e) => {
                if (e.key === 'Enter') {
                    document.getElementById('send-chat-btn').onclick();
                }
            };
        },
        chat_list: (payload) => renderChatList(payload),
        message_history: (payload) => {
            document.getElementById('chat-messages').innerHTML = '';
            payload.forEach(addChatMessage);
        },
        new_chat_message: (payload) => addChatMessage(payload),

        // Friend Requests & Invitations
        friend_list: (payload) => renderFriendList(payload),
        friend_request_sent: (payload) => showMessage(document.getElementById('add-friend-message-area'), payload, 'success'),
        friend_request_fail: (payload) => showMessage(document.getElementById('add-friend-message-area'), payload, 'error'),
        friend_requests: (payload) => renderFriendRequestList(payload),
        new_friend_request: (payload) => addFriendRequestToList(payload),
        friend_request_accepted: (payload) => {
            alert(payload);
            // Backend pushes updated lists to both clients now
        },
        friend_request_rejected: (payload) => {
            alert(payload);
            // Backend pushes updated list to the client now
        },
        invitation: (payload) => {
            if (confirm(`${payload.from_username} invites you to join the chat.`)) {
                sendWsMessage('join_room', { roomId: payload.room_id });
            }
        },
        voice_chat_invitation: (payload) => {
            if (confirm(`${payload.from_username} has started a voice chat. Join?`)) {
                const ws = getWebSocket();
                if (ws) {
                    startAudioCapture(ws).then(success => {
                        if (success) {
                            isVoiceActive = true;
                            voiceControls.classList.remove('hidden');
                            startVoiceBtn.classList.add('hidden');
                        }
                    });
                }
            }
        },

        // Admin
        admin_all_users: (payload) => renderUserList(payload),
        admin_all_rooms: (payload) => renderRoomList(payload),
        admin_create_user_ok: (payload) => alert(payload),
        admin_create_user_fail: (payload) => alert(`Error: ${payload}`),
        admin_generic_ok: (payload) => alert(payload),
        admin_error: (payload) => alert(`Error: ${payload}`),

        // General
        error: (e) => {
            console.error('[WS] WebSocket error:', e);
            showMessage(document.getElementById('message-area'), 'Connection error.');
        },
        close: (e) => {
            console.log(`[WS] WebSocket disconnected. Code: ${e.code}, Reason: ${e.reason}`);
        }
    };

    // --- Event Listeners ---

    loginBtn.addEventListener('click', () => handleAuth('login', handlers));
    registerBtn.addEventListener('click', () => handleAuth('register', handlers));

    adminPanelBtn.addEventListener('click', () => {
        mainView.classList.add('hidden');
        adminPanelView.classList.remove('hidden');
    });

    closeAdminPanelBtn.addEventListener('click', () => {
        adminPanelView.classList.add('hidden');
        mainView.classList.remove('hidden');
    });

    refreshUsersBtn.addEventListener('click', () => sendWsMessage('admin_get_all_users'));
    refreshRoomsBtn.addEventListener('click', () => sendWsMessage('admin_get_all_rooms'));

    createUserBtn.addEventListener('click', () => {
        const username = newUsernameInput.value;
        const password = newPasswordInput.value;
        const role = newUserRoleSelect.value;
        if (username && password) {
            sendWsMessage('admin_create_user', { username, password, role });
            newUsernameInput.value = '';
            newPasswordInput.value = '';
        } else {
            alert('New username and password cannot be empty.');
        }
    });

    shutdownServerBtn.addEventListener('click', () => {
        if (confirm('Are you sure you want to shut down the entire server?')) {
            sendWsMessage('admin_shutdown_server');
            alert('Shutdown command sent. The server will now terminate.');
        }
    });

    userListContainer.addEventListener('click', (e) => {
        const target = e.target.closest('[data-action="delete-user"]');
        if (!target) return;

        const userId = parseInt(target.dataset.userId, 10);
        const username = target.dataset.username;
        if (confirm(`Are you sure you want to delete user '${username}' (ID: ${userId})? This is irreversible.`)) {
            sendWsMessage('admin_delete_user', { user_id: userId });
        }
    });

    roomListContainer.addEventListener('click', (e) => {
        const target = e.target.closest('[data-action="delete-room"]');
        if (!target) return;

        const roomId = parseInt(target.dataset.roomId, 10);
        if (confirm(`Are you sure you want to delete room ID: ${roomId}? This is irreversible.`)) {
            sendWsMessage('admin_delete_room', { room_id: roomId });
        }
    });

    addFriendBtn.addEventListener('click', () => {
        const addFriendUsernameInput = document.getElementById('add-friend-username-input');
        const username = addFriendUsernameInput.value;
        if (username) {
            sendWsMessage('send_friend_request', { username });
            addFriendUsernameInput.value = '';
        }
    });

    logoutBtn.addEventListener('click', () => {
        localStorage.removeItem('authToken');
        window.location.reload();
    });

    chatList.addEventListener('click', (e) => {
        const chatItem = e.target.closest('.chat-list-item');
        if (chatItem) {
            const roomId = parseInt(chatItem.dataset.roomId, 10);
            if (roomId) {
                sendWsMessage('join_room', { roomId });
            }
        }
    });

    friendList.addEventListener('click', (e) => {
        const deleteBtn = e.target.closest('.delete-friend-btn');
        const friendItem = e.target.closest('.friend-list-item');

        if (deleteBtn) {
            const friendId = parseInt(deleteBtn.dataset.friendId, 10);
            const friendUsername = friendItem.dataset.friendUsername;
            if (confirm(`Are you sure you want to remove ${friendUsername} as a friend?`)) {
                sendWsMessage('delete_friend', { friendId });
            }
        } else if (friendItem) {
            const friendId = parseInt(friendItem.dataset.friendId, 10);
            if (friendId) {
                sendWsMessage('quick_chat_with_friend', { friendId });
            }
        }
    });

    backToChatsBtn.addEventListener('click', () => {
        callView.classList.add('hidden');
        mainView.classList.remove('hidden');
        stopAudioCapture();
        isVoiceActive = false;
    });

    startVoiceBtn.addEventListener('click', async () => {
        const ws = getWebSocket();
        if (!ws) return;

        // 1. Start your own audio first.
        isVoiceActive = await startAudioCapture(ws);
        if (isVoiceActive) {
            voiceControls.classList.remove('hidden');
            startVoiceBtn.classList.add('hidden');
            // 2. Then, send the invitation to others.
            sendWsMessage('request_voice_chat');
        }
    });

    muteMicBtn.addEventListener('click', () => {
        isMuted = !isMuted;
        setMute(isMuted); // Inform the audio module
        muteMicBtn.textContent = isMuted ? 'Unmute Mic' : 'Mute Mic';
        console.log("Mute status:", isMuted);
    });

    // --- Auto-Login on Page Load ---
    const token = localStorage.getItem('authToken');
    if (token) {
        initWebSocket(() => {
            sendWsMessage('auth_with_token', { token });
        }, handlers);
    } else {
        setupView.classList.remove('hidden');
    }
});