import { sendWsMessage } from './websocket.js';
import { t } from './i18n.js';

function renderFriendList(friends) {
    const friendList = document.getElementById('friend-list');
    friendList.innerHTML = ''; // Clear existing list
    if (friends && friends.length > 0) {
        friends.forEach(friend => {
            const friendItem = document.createElement('li');
            friendItem.className = 'friend-list-item';
            friendItem.dataset.friendId = friend.id;
            friendItem.dataset.friendUsername = friend.username;

            friendItem.innerHTML = `
                <div class="friend-name">
                    ${friend.username}
                    <span class="status ${friend.is_online ? 'online' : 'offline'}">
                        ${friend.is_online ? t('statusOnline') : t('statusOffline')}
                    </span>
                </div>
                <button class="delete-friend-btn btn-danger btn-small">${t('removeButton')}</button>
            `;
            friendList.appendChild(friendItem);
        });
    } else {
        friendList.innerHTML = `<li class="no-friends" data-i18n="noFriends">${t('noFriends')}</li>`;
    }
}

function renderFriendRequestList(requests) {
    const friendRequestList = document.getElementById('friend-request-list');
    friendRequestList.innerHTML = ''; // Clear existing list
    if (requests && requests.length > 0) {
        requests.forEach(req => addFriendRequestToList(req, false));
    } else {
        friendRequestList.innerHTML = `<li class="no-requests" data-i18n="noFriendRequests">${t('noFriendRequests')}</li>`;
    }
}

function addFriendRequestToList(req, isNew = true) {
    const friendRequestList = document.getElementById('friend-request-list');
    const noRequestsMessage = friendRequestList.querySelector('.no-requests');
    if (noRequestsMessage) {
        noRequestsMessage.remove();
    }

    const existingItem = friendRequestList.querySelector(`[data-request-id="${req.id}"]`);
    if (existingItem) return; // Avoid duplicates

    const requestItem = document.createElement('li');
    requestItem.dataset.requestId = req.id;

    let buttonsHtml;
    if (req.status === 'pending') {
        if (req.is_sender) {
            buttonsHtml = `<span class="request-status">${t('friendRequestSent')}</span>`;
        } else {
            buttonsHtml = `
                <button class="accept-friend-btn btn-safe btn-small">${t('acceptButton')}</button>
                <button class="reject-friend-btn btn-danger btn-small">${t('rejectButton')}</button>
            `;
        }
    } 

    requestItem.innerHTML = `
        <span>${req.from_username}</span>
        <div class="button-group">${buttonsHtml}</div>
    `;

    if (isNew) {
        friendRequestList.prepend(requestItem);
    } else {
        friendRequestList.appendChild(requestItem);
    }

    if (req.status === 'pending' && !req.is_sender) {
        requestItem.querySelector('.accept-friend-btn').addEventListener('click', () => {
            sendWsMessage('respond_to_friend_request', { requestId: req.id, accept: true });
            requestItem.remove();
        });
        requestItem.querySelector('.reject-friend-btn').addEventListener('click', () => {
            sendWsMessage('respond_to_friend_request', { requestId: req.id, accept: false });
            requestItem.remove();
        });
    }
}

export { renderFriendList, renderFriendRequestList, addFriendRequestToList };