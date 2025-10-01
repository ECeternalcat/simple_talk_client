import { sendWsMessage } from './websocket.js';

function addFriendRequestToList(req) {
    const friendRequestList = document.getElementById('friend-request-list');

    // Remove the 'no new requests' message if it exists
    const placeholder = friendRequestList.querySelector('.no-requests-placeholder');
    if (placeholder) {
        placeholder.remove();
    }

    const li = document.createElement('li');
    li.id = `friend-request-${req.id}`;
    li.innerHTML = `
        <span>${req.from_username}</span>
        <div>
            <button class="accept-friend-btn btn-admin btn-safe" data-requestid="${req.id}">Accept</button>
            <button class="reject-friend-btn btn-admin btn-danger" data-requestid="${req.id}">Reject</button>
        </div>
    `;
    friendRequestList.prepend(li); // Add to the top of the list

    // Re-add event listeners to the new buttons
    li.querySelector('.accept-friend-btn').addEventListener('click', (e) => {
        const requestId = parseInt(e.target.dataset.requestid, 10);
        sendWsMessage('respond_to_friend_request', { requestId, accept: true });
    });

    li.querySelector('.reject-friend-btn').addEventListener('click', (e) => {
        const requestId = parseInt(e.target.dataset.requestid, 10);
        sendWsMessage('respond_to_friend_request', { requestId, accept: false });
    });
}

function renderFriendRequestList(requests) {
    const friendRequestList = document.getElementById('friend-request-list');
    friendRequestList.innerHTML = '';
    if (requests.length === 0) {
        friendRequestList.innerHTML = '<li class="no-requests-placeholder">No new friend requests.</li>';
        return;
    }
    requests.forEach(addFriendRequestToList);
}

function renderFriendList(friends) {
    const friendList = document.getElementById('friend-list');
    friendList.innerHTML = '';
    if (friends.length === 0) {
        friendList.innerHTML = '<li>You have no friends yet.</li>';
        return;
    }

    friends.forEach(friend => {
        const li = document.createElement('li');
        li.className = 'friend-list-item';
        li.dataset.friendId = friend.id;
        li.dataset.friendUsername = friend.username;
        li.innerHTML = `
            <span class="friend-name">${friend.username}</span>
            <div class="friend-controls">
                <span class="status ${friend.is_online ? 'online' : 'offline'}"></span>
                <button class="delete-friend-btn btn-admin btn-danger" data-friend-id="${friend.id}">Delete</button>
            </div>
        `;
        friendList.appendChild(li);
    });
}

export { renderFriendRequestList, addFriendRequestToList, renderFriendList };