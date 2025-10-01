import { handleAuth } from './auth.js';
import { initWebSocket, sendWsMessage, setCurrentUser } from './websocket.js';
import { showMessage, addChatMessage, showPage } from './ui.js';
import { renderChatList } from './chats.js';
import { renderFriendRequestList, addFriendRequestToList, renderFriendList } from './friends.js';
import { renderUserList, renderRoomList } from './admin.js';
import { startAudioCapture, stopAudioCapture, setMute } from './audio.js';
import { initI18n, setLanguage, t } from './i18n.js';

document.addEventListener('DOMContentLoaded', async () => {
    // --- Views & Pages ---
    const setupView = document.getElementById('setup-view');
    const mainView = document.getElementById('main-view');
    const callView = document.getElementById('call-view');
    const adminPanelView = document.getElementById('admin-panel-view');

    // --- Buttons and Inputs ---
    const loginBtn = document.getElementById('login-btn');
    const registerBtn = document.getElementById('register-btn');
    const addFriendBtn = document.getElementById('add-friend-btn');
    const logoutBtn = document.getElementById('logout-btn');
    const backToMainBtn = document.getElementById('back-to-main-btn');
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
    const createUserBtn = document.getElementById('create-user-btn');
    const shutdownServerBtn = document.getElementById('shutdown-server-btn');
    const changePortBtn = document.getElementById('change-port-btn');
    const userListContainer = document.getElementById('user-list-container');
    const roomListContainer = document.getElementById('room-list-container');
    const languageSelector = document.getElementById('language-selector');

    // --- Nav Buttons ---
    const navChatsBtn = document.getElementById('nav-chats');
    const navFriendsBtn = document.getElementById('nav-friends');
    const navProfileBtn = document.getElementById('nav-profile');

    // --- Profile Page ---
    const profileUsername = document.getElementById('profile-username');

    // --- App State ---
    let isMuted = false;
    let isVoiceActive = false;
    let isUserAuthenticated = false;
    let lastChatList = [];
    let lastFriendList = [];
    let lastFriendRequestList = [];

    // Initialize i18n
    await initI18n();

    function rerenderDynamicLists() {
        renderChatList(lastChatList);
        renderFriendList(lastFriendList);
        renderFriendRequestList(lastFriendRequestList);
    }

    // --- WebSocket Message Handlers ---
    const handlers = {
        // Auth
        register_ok: (payload) => showMessage(document.getElementById('message-area'), t('genericSuccess').replace('{message}', payload), 'success'),
        register_fail: (payload) => showMessage(document.getElementById('message-area'), t('genericError').replace('{message}', payload), 'error'),
        auth_fail: (payload) => {
            isUserAuthenticated = false;
            localStorage.removeItem('authToken');
            showMessage(document.getElementById('message-area'), t('genericError').replace('{message}', payload), 'error');
            mainView.classList.add('hidden');
            callView.classList.add('hidden');
            setupView.classList.remove('hidden');
        },
        auth_ok: (payload) => {
            if (payload.token) {
                localStorage.setItem('authToken', payload.token);
            }
            setCurrentUser(payload);
            isUserAuthenticated = true; // Set authentication flag

            setupView.classList.add('hidden');
            mainView.classList.remove('hidden');
            showPage('chats-page');

            profileUsername.textContent = payload.username;
            if (payload.role === 'admin') {
                adminPanelBtn.classList.remove('hidden');
            }

            // Render any lists that were received before authentication was complete
            rerenderDynamicLists();
        },

        // Room & Chat
        join_ok: (payload) => {
            mainView.classList.add('hidden');
            callView.classList.remove('hidden');
            document.getElementById('status-text').textContent = `${t('tableHeaderId')}: ${payload.roomId}`;
            
            voiceControls.classList.add('hidden');
            startVoiceBtn.classList.remove('hidden');
            isVoiceActive = false;
            stopAudioCapture();

            const currentRoomId = payload.roomId;
            chatInput.dataset.currentRoomId = currentRoomId;
            sendChatBtn.onclick = () => {
                 const content = chatInput.value;
                 if (content) {
                    sendWsMessage('send_chat_message', { roomId: currentRoomId, content });
                    chatInput.value = '';
                 }
            };
             chatInput.onkeyup = (e) => {
                if (e.key === 'Enter') {
                    sendChatBtn.onclick();
                }
            };
        },
        chat_list: (payload) => {
            lastChatList = payload;
            if (isUserAuthenticated) {
                renderChatList(lastChatList);
            }
        },
        message_history: (payload) => {
            document.getElementById('chat-messages').innerHTML = '';
            payload.forEach(addChatMessage);
        },
        new_chat_message: (payload) => addChatMessage(payload),

        // Friend Requests & Invitations
        friend_list: (payload) => {
            lastFriendList = payload;
            if (isUserAuthenticated) {
                renderFriendList(lastFriendList);
            }
        },
        friend_request_sent: (payload) => showMessage(document.getElementById('add-friend-message-area'), t('friendRequestSentSuccess').replace('{username}', payload.username), 'success'),
        friend_request_fail: (payload) => showMessage(document.getElementById('add-friend-message-area'), t('friendRequestFail').replace('{error}', payload), 'error'),
        friend_requests: (payload) => {
            lastFriendRequestList = payload;
            if (isUserAuthenticated) {
                renderFriendRequestList(lastFriendRequestList);
            }
        },
        new_friend_request: (payload) => addFriendRequestToList(payload),
        friend_request_accepted: (payload) => {
            alert(t('friendRequestAccepted').replace('{username}', payload.from_username));
            sendWsMessage('get_friend_list');
            sendWsMessage('get_friend_requests');
        },
        friend_request_rejected: (payload) => {
            alert(t('friendRequestRejected').replace('{username}', payload.from_username));
        },
        invitation: (payload) => {
            if (confirm(t('chatInvitation').replace('{username}', payload.from_username))) {
                sendWsMessage('join_room', { roomId: payload.room_id });
            }
        },
        voice_chat_invitation: (payload) => {
            if (confirm(t('voiceInvitation').replace('{username}', payload.from_username))) {
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
        admin_create_user_ok: (payload) => alert(t('genericSuccess').replace('{message}', payload)),
        admin_create_user_fail: (payload) => alert(t('genericError').replace('{message}', payload)),
        admin_change_port_ok: () => alert(t('changePortSuccess')),
        admin_change_port_fail: (payload) => alert(t('changePortFail').replace('{error}', payload)),
        admin_generic_ok: (payload) => alert(t('genericSuccess').replace('{message}', payload)),
        admin_error: (payload) => alert(t('genericError').replace('{message}', payload)),

        // General
        error: (e) => {
            console.error('[WS] WebSocket error:', e);
            showMessage(document.getElementById('message-area'), t('connectionError'));
        },
        close: (e) => {
            console.log(`[WS] WebSocket disconnected. Code: ${e.code}, Reason: ${e.reason}`);
        }
    };

    // --- Event Listeners ---

    loginBtn.addEventListener('click', () => handleAuth('login', handlers));
    registerBtn.addEventListener('click', () => handleAuth('register', handlers));

    languageSelector.addEventListener('change', async (e) => {
        await setLanguage(e.target.value);
        rerenderDynamicLists();
    });

    // Navigation
    navChatsBtn.addEventListener('click', () => showPage('chats-page'));
    navFriendsBtn.addEventListener('click', () => showPage('friends-page'));
    navProfileBtn.addEventListener('click', () => showPage('profile-page'));

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
        const newUsernameInput = document.getElementById('new-username-input');
        const newPasswordInput = document.getElementById('new-password-input');
        const newUserRoleSelect = document.getElementById('new-user-role-select');
        const username = newUsernameInput.value;
        const password = newPasswordInput.value;
        const role = newUserRoleSelect.value;
        if (username && password) {
            sendWsMessage('admin_create_user', { username, password, role });
            newUsernameInput.value = '';
            newPasswordInput.value = '';
        } else {
            alert(t('adminNewUserEmptyFields'));
        }
    });

    shutdownServerBtn.addEventListener('click', () => {
        if (confirm(t('confirmShutdown'))) {
            sendWsMessage('admin_shutdown_server');
            alert(t('shutdownCommandSent'));
        }
    });

    changePortBtn.addEventListener('click', () => {
        const newPortInput = document.getElementById('new-port-input');
        const port = parseInt(newPortInput.value, 10);

        if (isNaN(port) || port < 1 || port > 65535) {
            alert(t('invalidPort'));
            return;
        }

        sendWsMessage('admin_change_port', { port });
        newPortInput.value = '';
    });

    userListContainer.addEventListener('click', (e) => {
        const target = e.target.closest('[data-action="delete-user"]');
        if (!target) return;

        const userId = parseInt(target.dataset.userId, 10);
        const username = target.dataset.username;
        if (confirm(t('confirmDeleteUser').replace('{username}', username).replace('{userId}', userId))) {
            sendWsMessage('admin_delete_user', { user_id: userId });
        }
    });

    roomListContainer.addEventListener('click', (e) => {
        const target = e.target.closest('[data-action="delete-room"]');
        if (!target) return;

        const roomId = parseInt(target.dataset.roomId, 10);
        if (confirm(t('confirmDeleteRoom').replace('{roomId}', roomId))) {
            sendWsMessage('admin_delete_room', { room_id: roomId });
        }
    });

    // Admin Panel Accordion
    adminPanelView.addEventListener('click', (e) => {
        const header = e.target.closest('.accordion-header');
        if (!header) return;

        const currentlyActive = document.querySelector('.accordion-header.active');
        if (currentlyActive && currentlyActive !== header) {
            currentlyActive.classList.remove('active');
        }

        header.classList.toggle('active');
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
            const friendId = parseInt(friendItem.dataset.friendId, 10);
            const friendUsername = friendItem.dataset.friendUsername;
            if (confirm(t('confirmRemoveFriend').replace('{username}', friendUsername))) {
                sendWsMessage('delete_friend', { friendId });
                friendItem.remove();
            }
        } else if (friendItem) {
            const friendId = parseInt(friendItem.dataset.friendId, 10);
            if (friendId) {
                sendWsMessage('quick_chat_with_friend', { friendId });
            }
        }
    });

    backToMainBtn.addEventListener('click', () => {
        callView.classList.add('hidden');
        mainView.classList.remove('hidden');
        stopAudioCapture();
        isVoiceActive = false;
    });

    startVoiceBtn.addEventListener('click', async () => {
        const ws = getWebSocket();
        if (!ws) return;

        isVoiceActive = await startAudioCapture(ws);
        if (isVoiceActive) {
            voiceControls.classList.remove('hidden');
            startVoiceBtn.classList.add('hidden');
            sendWsMessage('request_voice_chat');
        }
    });

    muteMicBtn.addEventListener('click', () => {
        isMuted = !isMuted;
        setMute(isMuted);
        muteMicBtn.textContent = isMuted ? t('unmuteMicButton') : t('muteMicButton');
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