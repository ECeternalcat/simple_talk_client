import { t } from './i18n.js';
import { currentUser } from './websocket.js';

function renderChatList(chats) {
    const chatList = document.getElementById('chat-list');
    chatList.innerHTML = ''; // Clear the list

    if (!chats || chats.length === 0) {
        const emptyItem = document.createElement('li');
        emptyItem.textContent = t('noActiveChats');
        chatList.appendChild(emptyItem);
        return;
    }

    chats.forEach(chat => {
        const chatItem = document.createElement('li');
        chatItem.className = 'chat-list-item';
        chatItem.dataset.roomId = chat.room_id;

        let displayName = chat.name; // Prioritize server-given name
        let participants = Array.isArray(chat.participants) ? chat.participants : [];

        // If no name is given by the server, generate one from participants
        if (!displayName && currentUser) {
            const otherParticipants = participants.filter(p => p !== currentUser.username);
            if (otherParticipants.length > 0) {
                displayName = otherParticipants.join(', ');
            } else if (participants.length === 1) {
                // It's a chat with only the user themselves
                displayName = participants[0]; // Just show their own name
            } else {
                displayName = t('unnamedChat'); // Fallback
            }
        }

        const membersString = participants.length > 0 ? t('chatMembers').replace('{members}', participants.join(', ')) : '';

        chatItem.innerHTML = `
            <div class="chat-name">${displayName || t('Unnamed Chat')}</div>
            <div class="chat-members">${membersString}</div>
        `;
        chatList.appendChild(chatItem);
    });
}

export { renderChatList };