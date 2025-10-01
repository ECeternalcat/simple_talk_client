import { t } from './i18n.js';

function renderUserList(users) {
    const userListContainer = document.getElementById('user-list-container');
    if (!userListContainer) return;

    if (!users || users.length === 0) {
        userListContainer.innerHTML = `<p>${t('noUsersFound')}</p>`;
        return;
    }

    const table = document.createElement('table');
    table.classList.add('responsive-table');
    table.innerHTML = `
        <thead>
            <tr>
                <th>${t('tableHeaderId')}</th>
                <th>${t('tableHeaderUsername')}</th>
                <th>${t('tableHeaderRole')}</th>
                <th>${t('tableHeaderActions')}</th>
            </tr>
        </thead>
        <tbody>
            ${users.map(user => `
                <tr>
                    <td data-label="${t('tableHeaderId')}">${user._id}</td>
                    <td data-label="${t('tableHeaderUsername')}">${user.username}</td>
                    <td data-label="${t('tableHeaderRole')}">${user.role}</td>
                    <td data-label="${t('tableHeaderActions')}">
                        <button class="btn-admin btn-danger btn-small" data-action="delete-user" data-user-id="${user._id}" data-username="${user.username}">${t('deleteButton')}</button>
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
        roomListContainer.innerHTML = `<p>${t('noActiveRooms')}</p>`;
        return;
    }

    const table = document.createElement('table');
    table.classList.add('responsive-table');
    table.innerHTML = `
        <thead>
            <tr>
                <th>${t('tableHeaderId')}</th>
                <th>${t('tableHeaderType')}</th>
                <th>${t('tableHeaderParticipants')}</th>
                <th>${t('tableHeaderCreatedAt')}</th>
                <th>${t('tableHeaderActions')}</th>
            </tr>
        </thead>
        <tbody>
            ${rooms.map(room => `
                <tr>
                    <td data-label="${t('tableHeaderId')}">${room.id}</td>
                    <td data-label="${t('tableHeaderType')}">${room.is_private ? 'Private' : 'Group'}</td>
                    <td data-label="${t('tableHeaderParticipants')}">${room.participants.join(', ')}</td>
                    <td data-label="${t('tableHeaderCreatedAt')}">${new Date(room.created_at).toLocaleString()}</td>
                    <td data-label="${t('tableHeaderActions')}">
                        <button class="btn-admin btn-danger btn-small" data-action="delete-room" data-room-id="${room.id}">${t('deleteButton')}</button>
                    </td>
                </tr>
            `).join('')}
        </tbody>
    `;
    roomListContainer.innerHTML = '';
    roomListContainer.appendChild(table);
}

export { renderUserList, renderRoomList };