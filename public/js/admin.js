function renderUserList(users) {
    const userListContainer = document.getElementById('user-list-container');
    if (!userListContainer) return;

    if (!users || users.length === 0) {
        userListContainer.innerHTML = '<p>No users found.</p>';
        return;
    }

    const table = document.createElement('table');
    table.innerHTML = `
        <thead>
            <tr>
                <th>ID</th>
                <th>Username</th>
                <th>Role</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
            ${users.map(user => `
                <tr>
                    <td>${user._id}</td>
                    <td>${user.username}</td>
                    <td>${user.role}</td>
                    <td>
                        <button class="btn-admin btn-danger btn-small" data-action="delete-user" data-user-id="${user._id}" data-username="${user.username}">Delete</button>
                    </td>
                </tr>
            `).join('')}
        </tbody>
    `;
    userListContainer.innerHTML = '';
    userListContainer.appendChild(table);
}

function renderRoomList(rooms) {
    const roomListContainer = document.getElementById('room-list-container');
    if (!roomListContainer) return;

    if (!rooms || rooms.length === 0) {
        roomListContainer.innerHTML = '<p>No active rooms found.</p>';
        return;
    }

    const table = document.createElement('table');
    table.innerHTML = `
        <thead>
            <tr>
                <th>ID</th>
                <th>Type</th>
                <th>Participants</th>
                <th>Created At</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
            ${rooms.map(room => `
                <tr>
                    <td>${room.id}</td>
                    <td>${room.is_private ? 'Private' : 'Group'}</td>
                    <td>${room.participants.join(', ')}</td>
                    <td>${new Date(room.created_at).toLocaleString()}</td>
                    <td>
                        <button class="btn-admin btn-danger btn-small" data-action="delete-room" data-room-id="${room.id}">Delete</button>
                    </td>
                </tr>
            `).join('')}
        </tbody>
    `;
    roomListContainer.innerHTML = '';
    roomListContainer.appendChild(table);
}

export { renderUserList, renderRoomList };