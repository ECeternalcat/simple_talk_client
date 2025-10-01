import { sendWsMessage } from './websocket.js';

function renderChatList(chats) {
    const chatList = document.getElementById('chat-list');
    chatList.innerHTML = '';
    if (chats.length === 0) {
        chatList.innerHTML = '<li>No active chats. Add a friend to start one!</li>';
        return;
    }

    chats.forEach(chat => {
        const li = document.createElement('li');
        li.className = 'chat-list-item';
        li.dataset.roomId = chat.room_id;

        const statusClass = chat.is_online ? 'online' : 'offline';

        li.innerHTML = `
            <span class="chat-name">${chat.room_name}</span>
            <span class="status ${statusClass}">●</span>
        `;
        chatList.appendChild(li);
    });
}

function renderFriendRequestList(requests) {
    const friendRequestList = document.getElementById('friend-request-list');
    friendRequestList.innerHTML = '';
    if (requests.length === 0) {
        friendRequestList.innerHTML = '<li>No new friend requests.</li>';
        return;
    }

    requests.forEach(req => {
        const li = document.createElement('li');
        li.innerHTML = `
            <span>${req.from_username} wants to be your friend.</span>
            <div>
                <button class="accept-friend-btn btn-admin btn-safe" data-requestid="${req.id}">Accept</button>
                <button class="reject-friend-btn btn-admin btn-danger" data-requestid="${req.id}">Reject</button>
            </div>
        `;
        friendRequestList.appendChild(li);
    });

    document.querySelectorAll('.accept-friend-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const requestId = parseInt(e.target.dataset.requestid, 10);
            sendWsMessage('respond_to_friend_request', { requestId, accept: true });
        });
    });

    document.querySelectorAll('.reject-friend-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const requestId = parseInt(e.target.dataset.requestid, 10);
            sendWsMessage('respond_to_friend_request', { requestId, accept: false });
        });
    });
}

export { renderChatList, renderFriendRequestList };
