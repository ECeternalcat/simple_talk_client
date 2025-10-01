use crate::{db, send_ws_message_to, AppState, Client, InvitationPayload, JoinRoomPayload, QuickChatPayload, RoomId, WsRequestMessage, ChatMessagePayload, SendFriendRequestPayload, RespondToFriendRequestPayload, AdminCreateUserPayload, AdminDeleteUserPayload, AdminDeleteRoomPayload, DeleteFriendPayload};
use axum::extract::ws::Message;
use rusqlite::params;
use std::sync::Arc;
use tokio::sync::mpsc;

// --- Standalone Handlers (called from main) ---

/// Fetches all rooms for the user and sends it to them.
pub async fn handle_get_user_rooms(
    state: Arc<AppState>,
    user: &db::User,
    own_tx: &mpsc::UnboundedSender<Message>,
) {
    let conn = state.db_pool.get().unwrap();
    match db::get_user_rooms(&conn, user._id) {
        Ok(rooms) => {
            let online_users = state.online_users.lock().unwrap();
            let chat_list: Vec<db::RoomInfo> = rooms
                .into_iter()
                .map(|(room_id, room_name)| db::RoomInfo {
                    room_id,
                    room_name,
                    // Check if the other user is online.
                    // This is a bit inefficient, a user_id map would be better.
                    is_online: conn.query_row(
                        "SELECT u.id FROM users u JOIN room_participants rp ON u.id = rp.user_id WHERE rp.room_id = ?1 AND rp.user_id != ?2",
                        params![room_id, user._id],
                        |row| row.get::<_, i32>(0)
                    ).ok().map_or(false, |id| online_users.contains_key(&id))
                })
                .collect();

            let resp = crate::WsResponseMessage {
                r#type: "chat_list".to_string(),
                payload: serde_json::json!(chat_list),
            };
            let _ = own_tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
        }
        Err(e) => {
            tracing::error!("Failed to get user rooms: {}", e);
        }
    }
}

/// Fetches all pending friend requests for the user and sends it to them.
pub async fn handle_get_friend_requests(
    state: Arc<AppState>,
    user: &db::User,
    own_tx: &mpsc::UnboundedSender<Message>,
) {
    let conn = state.db_pool.get().unwrap();
    match db::get_friend_requests(&conn, user._id) {
        Ok(requests) => {
            let resp = crate::WsResponseMessage {
                r#type: "friend_requests".to_string(),
                payload: serde_json::json!(requests),
            };
            let _ = own_tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
        }
        Err(e) => {
            tracing::error!("Failed to get friend requests: {}", e);
        }
    }
}

/// Fetches all friends for the user and sends it to them.
pub async fn handle_get_friend_list(
    state: Arc<AppState>,
    user: &db::User,
    own_tx: &mpsc::UnboundedSender<Message>,
) {
    let conn = state.db_pool.get().unwrap();
    match db::get_friends(&conn, user._id) {
        Ok(friends) => {
            let online_users = state.online_users.lock().unwrap();
            let friend_list: Vec<crate::FriendInfo> = friends
                .into_iter()
                .map(|f| crate::FriendInfo {
                    id: f._id,
                    username: f.username,
                    is_online: online_users.contains_key(&f._id),
                })
                .collect();

            let resp = crate::WsResponseMessage {
                r#type: "friend_list".to_string(),
                payload: serde_json::json!(friend_list),
            };
            let _ = own_tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
        }
        Err(e) => {
            tracing::error!("Failed to get friend list: {}", e);
        }
    }
}


/// Handles all incoming text-based WebSocket messages.
pub async fn handle_message(
    req: WsRequestMessage,
    state: Arc<AppState>,
    user: &db::User,
    current_room_id: &mut Option<RoomId>,
    own_tx: &mpsc::UnboundedSender<Message>,
) {
    match req.r#type.as_str() {
        // --- Room Management ---
        "join_room" => {
            if let Ok(p) = serde_json::from_value::<JoinRoomPayload>(req.payload.clone()) {
                // 1. Add user to the in-memory room struct
                let client = Client { sender: own_tx.clone() };
                state.rooms.lock().unwrap().entry(p.room_id).or_default().clients.insert(user._id, client);
                *current_room_id = Some(p.room_id);

                // 2. Acknowledge join and send message history
                send_ws_message_to(own_tx, "join_ok", &serde_json::json!({ "roomId": p.room_id })).await;
                tracing::info!("User '{}' joined room '{}'", user.username, p.room_id);

                let conn = state.db_pool.get().unwrap();
                if let Ok(messages) = db::get_messages_for_room(&conn, p.room_id) {
                    let resp = crate::WsResponseMessage {
                        r#type: "message_history".to_string(),
                        payload: serde_json::json!(messages),
                    };
                    let _ = own_tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                }
            }
        }

        // --- Chatting ---
        "send_chat_message" => {
            if let (Some(room_id), Ok(p)) = (current_room_id, serde_json::from_value::<ChatMessagePayload>(req.payload.clone())) {
                if *room_id != p.room_id { return; } // Ensure user is sending to their current room

                let conn = state.db_pool.get().unwrap();
                if let Ok(message) = db::create_message(&conn, *room_id, &user.username, &p.content) {
                    let rooms = state.rooms.lock().unwrap();
                    if let Some(room) = rooms.get(room_id) {
                        let resp = crate::WsResponseMessage {
                            r#type: "new_chat_message".to_string(),
                            payload: serde_json::json!(message),
                        };
                        let resp_text = serde_json::to_string(&resp).unwrap();
                        // Broadcast to all clients in the room
                        for client in room.clients.values() {
                            let _ = client.sender.send(Message::Text(resp_text.clone()));
                        }
                    }
                }
            }
        }

        // --- Friend & Chat Creation ---
        "get_chat_list" => {
            handle_get_user_rooms(state.clone(), user, own_tx).await;
        }
        "get_friend_list" => {
            handle_get_friend_list(state.clone(), user, own_tx).await;
        }
        "quick_chat_with_friend" => {
            if let Ok(p) = serde_json::from_value::<QuickChatPayload>(req.payload.clone()) {
                let mut conn = state.db_pool.get().unwrap();
                match db::get_or_create_private_room(&mut conn, user._id, p.friend_id) {
                    Ok(room_id) => {
                        let friend_is_online;
                        let friend_tx = {
                            let online_users = state.online_users.lock().unwrap();
                            friend_is_online = online_users.contains_key(&p.friend_id);
                            online_users.get(&p.friend_id).cloned()
                        };

                        // Always take the user to the room.
                        // 1. Add user to the in-memory room struct
                        let client = Client { sender: own_tx.clone() };
                        state.rooms.lock().unwrap().entry(room_id).or_default().clients.insert(user._id, client);
                        *current_room_id = Some(room_id);

                        // 2. Acknowledge join and send message history
                        send_ws_message_to(own_tx, "join_ok", &serde_json::json!({ "roomId": room_id })).await;
                        if let Ok(messages) = db::get_messages_for_room(&conn, room_id) {
                             let resp = crate::WsResponseMessage {
                                r#type: "message_history".to_string(),
                                payload: serde_json::json!(messages),
                            };
                            let _ = own_tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                        }

                        // If friend is online, invite them.
                        if friend_is_online {
                            if let Some(friend_tx) = friend_tx {
                                let invitation = InvitationPayload {
                                    from_username: user.username.clone(),
                                    room_id,
                                    room_name: user.username.clone(), // The room is named after the inviting user
                                };
                                send_ws_message_to(&friend_tx, "invitation", invitation).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get or create private room: {}", e);
                    }
                }
            }
        }
        "send_friend_request" => {
            if let Ok(p) = serde_json::from_value::<SendFriendRequestPayload>(req.payload.clone()) {
                let conn = state.db_pool.get().unwrap();
                let mut op_success = false;
                let mut info_to_send: Option<db::FriendRequestInfo> = None;

                if let Ok(target_user) = db::get_user_by_username(&conn, &p.username) {
                    if target_user._id != user._id {
                        match db::send_friend_request(&conn, user._id, target_user._id) {
                            Ok(info) => {
                                op_success = true;
                                info_to_send = Some(info);
                            },
                            Err(e) => {
                                tracing::error!("DB error in send_friend_request: {}", e);
                                op_success = false;
                            }
                        }
                    }
                }

                if op_success {
                    if let Some(info) = info_to_send {
                        match info.status.as_str() {
                            "pending" => {
                                send_ws_message_to(own_tx, "friend_request_sent", "Friend request sent.").await;

                                let friend_tx = {
                                    let online_users_map = state.online_users.lock().unwrap();
                                    online_users_map.get(&info.to_user_id).cloned()
                                };

                                if let Some(friend_tx) = friend_tx {
                                    send_ws_message_to(&friend_tx, "new_friend_request", info).await;
                                }
                            }
                            "accepted" => {
                                send_ws_message_to(own_tx, "friend_request_fail", "You are already friends with this user.").await;
                            }
                            _ => {
                                send_ws_message_to(own_tx, "friend_request_fail", "Cannot send friend request at this time.").await;
                            }
                        }
                    }
                } else {
                    send_ws_message_to(own_tx, "friend_request_fail", "User not found, or you cannot send a request to yourself.").await;
                }
            }
        }
        "respond_to_friend_request" => {
            if let Ok(p) = serde_json::from_value::<RespondToFriendRequestPayload>(req.payload.clone()) {
                if p.accept {
                    let (op_success, sender_id, sender_user) = {
                        let mut conn = state.db_pool.get().unwrap();
                        let sender_id_res: Result<i32, _> = conn.query_row(
                            "SELECT from_user_id FROM friend_requests WHERE id = ?1",
                            params![p.request_id],
                            |row| row.get(0),
                        );

                        if let Ok(sender_id) = sender_id_res {
                            if db::accept_friend_request(&mut conn, p.request_id).is_ok() {
                                let sender_user_res = conn.query_row(
                                    "SELECT id, username, password_hash, role FROM users WHERE id = ?1",
                                    params![sender_id],
                                    |row| Ok(db::User { _id: row.get(0)?, username: row.get(1)?, password_hash: row.get(2)?, role: row.get(3)? })
                                );
                                (true, Some(sender_id), sender_user_res.ok())
                            } else {
                                (false, None, None) // accept_friend_request failed
                            }
                        } else {
                            (false, None, None) // query_row for sender_id failed
                        }
                    };

                    if op_success {
                        // --- Notify self (the acceptor) ---
                        send_ws_message_to(own_tx, "friend_request_accepted", "Friend request accepted. A new chat has been created.").await;
                        handle_get_user_rooms(state.clone(), user, own_tx).await;
                        handle_get_friend_requests(state.clone(), user, own_tx).await;
                        handle_get_friend_list(state.clone(), user, own_tx).await;

                        // --- Notify the original sender ---
                        if let (Some(id), Some(s_user)) = (sender_id, sender_user) {
                            let sender_tx = {
                                let online_users = state.online_users.lock().unwrap();
                                online_users.get(&id).cloned()
                            };

                            if let Some(s_tx) = sender_tx {
                                tracing::info!("Notifying original sender '{}' of accepted request.", s_user.username);
                                handle_get_user_rooms(state.clone(), &s_user, &s_tx).await;
                                handle_get_friend_list(state.clone(), &s_user, &s_tx).await;
                            }
                        }
                    } else {
                        send_ws_message_to(own_tx, "friend_request_fail", "Database operation failed.").await;
                    }
                } else {
                    let conn = state.db_pool.get().unwrap();
                    match db::reject_friend_request(&conn, p.request_id) {
                        Ok(_) => {
                            send_ws_message_to(own_tx, "friend_request_rejected", "Friend request rejected.").await;
                            handle_get_friend_requests(state, user, own_tx).await; // Refresh the list
                        }
                        Err(e) => {
                            send_ws_message_to(own_tx, "friend_request_fail", e.to_string()).await;
                        }
                    }
                }
            }
        }
        "delete_friend" => {
            if let Ok(p) = serde_json::from_value::<DeleteFriendPayload>(req.payload.clone()) {
                let mut conn = state.db_pool.get().unwrap();
                match db::delete_friend(&mut conn, user._id, p.friend_id) {
                    Ok(_) => {
                        // Notify self
                        handle_get_friend_list(state.clone(), user, own_tx).await;
                        handle_get_user_rooms(state.clone(), user, own_tx).await; // Also refresh chats

                        // Notify the other user if they are online
                        let friend_tx = {
                            let online_users = state.online_users.lock().unwrap();
                            online_users.get(&p.friend_id).cloned()
                        };
                        if let Some(friend_tx) = friend_tx {
                            // We need the other user's User object to refresh their lists
                            let other_user: Option<db::User> = conn.query_row(
                                "SELECT id, username, password_hash, role FROM users WHERE id = ?1",
                                params![p.friend_id],
                                |row| Ok(db::User { _id: row.get(0)?, username: row.get(1)?, password_hash: row.get(2)?, role: row.get(3)? })
                            ).ok();
                            
                            if let Some(ou) = other_user {
                                handle_get_friend_list(state.clone(), &ou, &friend_tx).await;
                                handle_get_user_rooms(state.clone(), &ou, &friend_tx).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete friend: {}", e);
                    }
                }
            }
        }

        // --- Admin commands (can be added back here if needed) ---

        "request_voice_chat" => {
            if let Some(ref room_id) = current_room_id {
                let invitation = crate::VoiceChatInvitationPayload {
                    from_username: user.username.clone(),
                };

                let peer_txs: Vec<_> = { // Scope for locks
                    let rooms = state.rooms.lock().unwrap();
                    if let Some(room) = rooms.get(room_id) {
                        let online_users = state.online_users.lock().unwrap();
                        room.clients.iter()
                            .filter_map(|(peer_id, _)| {
                                if *peer_id != user._id {
                                    online_users.get(peer_id).cloned()
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                };

                for peer_tx in peer_txs {
                    tracing::info!("Sending voice chat invitation");
                    send_ws_message_to(&peer_tx, "voice_chat_invitation", &invitation).await;
                }
            }
        }

        // --- Admin commands ---
        "admin_get_all_users" => {
            if user.role != "admin" { return; }
            let conn = state.db_pool.get().unwrap();
            match db::get_all_users(&conn) {
                Ok(users) => {
                    send_ws_message_to(own_tx, "admin_all_users", users).await;
                }
                Err(e) => {
                    tracing::error!("Failed to get all users for admin: {}", e);
                    send_ws_message_to(own_tx, "admin_error", "Failed to retrieve users.").await;
                }
            }
        }
        "admin_get_all_rooms" => {
            if user.role != "admin" { return; }
            let conn = state.db_pool.get().unwrap();
            match db::get_all_rooms(&conn) {
                Ok(rooms) => {
                    send_ws_message_to(own_tx, "admin_all_rooms", rooms).await;
                }
                Err(e) => {
                    tracing::error!("Failed to get all rooms for admin: {}", e);
                    send_ws_message_to(own_tx, "admin_error", "Failed to retrieve rooms.").await;
                }
            }
        }
        "admin_create_user" => {
            if user.role != "admin" { return; }
            if let Ok(p) = serde_json::from_value::<AdminCreateUserPayload>(req.payload.clone()) {
                let conn = state.db_pool.get().unwrap();
                match db::create_user(&conn, &p.username, &p.password, Some(&p.role)) {
                    Ok(_) => {
                        send_ws_message_to(own_tx, "admin_create_user_ok", "User created successfully.").await;
                        // Also refresh the user list
                        let users = db::get_all_users(&conn).unwrap_or_default();
                        send_ws_message_to(own_tx, "admin_all_users", users).await;
                    }
                    Err(e) => {
                        send_ws_message_to(own_tx, "admin_create_user_fail", e.to_string()).await;
                    }
                }
            }
        }
        "admin_shutdown_server" => {
            if user.role != "admin" { return; }
            tracing::warn!("Shutdown request received from admin: {}", user.username);
            if let Some(tx) = state.shutdown_tx.lock().unwrap().take() {
                if tx.send(()).is_err() {
                    tracing::error!("Failed to send shutdown signal.");
                }
            }
        }
        "admin_delete_user" => {
            if user.role != "admin" { return; }
            if let Ok(p) = serde_json::from_value::<AdminDeleteUserPayload>(req.payload.clone()) {
                let conn = state.db_pool.get().unwrap();
                match db::delete_user(&conn, p.user_id) {
                    Ok(_) => {
                        send_ws_message_to(own_tx, "admin_generic_ok", "User deleted successfully.").await;
                        let users = db::get_all_users(&conn).unwrap_or_default();
                        send_ws_message_to(own_tx, "admin_all_users", users).await;
                    }
                    Err(e) => {
                        send_ws_message_to(own_tx, "admin_error", e.to_string()).await;
                    }
                }
            }
        }
        "admin_delete_room" => {
            if user.role != "admin" { return; }
            if let Ok(p) = serde_json::from_value::<AdminDeleteRoomPayload>(req.payload.clone()) {
                let conn = state.db_pool.get().unwrap();
                match db::delete_room(&conn, p.room_id) {
                    Ok(_) => {
                        send_ws_message_to(own_tx, "admin_generic_ok", "Room deleted successfully.").await;
                        let rooms = db::get_all_rooms(&conn).unwrap_or_default();
                        send_ws_message_to(own_tx, "admin_all_rooms", rooms).await;
                    }
                    Err(e) => {
                        send_ws_message_to(own_tx, "admin_error", e.to_string()).await;
                    }
                }
            }
        }

        _ => {}
    }
}