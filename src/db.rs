use lazy_static::lazy_static;
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension, Result};
use serde::Serialize;
use uuid::Uuid;

// A type alias for the connection pool.
pub type Pool = r2d2::Pool<SqliteConnectionManager>;

// A type alias for a single connection from the pool.
pub type Connection = r2d2::PooledConnection<SqliteConnectionManager>;

// --- Data Structs ---

#[derive(Debug, Serialize, Clone)]
pub struct User {
    pub _id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub id: i32,
    pub room_id: i64,
    pub sender_username: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct FriendRequestInfo {
    pub id: i32,
    pub from_user_id: i32,
    pub from_username: String,
    pub to_user_id: i32,
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub room_id: i64,
    pub name: Option<String>,
    pub participants: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminRoomInfo {
    pub id: i64,
    pub name: Option<String>,
    pub is_private: bool,
    pub created_at: String,
    pub participants: Vec<String>, // Usernames
}

lazy_static! {
    pub static ref DB_POOL: Pool = {
        let manager = SqliteConnectionManager::file("app.db");
        r2d2::Pool::new(manager).expect("Failed to create DB pool.")
    };
}

/// Initializes the database and creates tables if they don't exist.
pub fn init_db() -> Result<()> {
    let conn = DB_POOL.get().expect("Failed to get DB connection from pool.");

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (\n            id              INTEGER PRIMARY KEY,\n            username        TEXT NOT NULL UNIQUE,\n            password_hash   TEXT NOT NULL,\n            role            TEXT NOT NULL DEFAULT 'normal'\n        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS rooms (\n            id              INTEGER PRIMARY KEY AUTOINCREMENT,\n            name            TEXT, -- For group chats in the future
            is_private      BOOLEAN NOT NULL DEFAULT TRUE, -- To distinguish 1-on-1 chats
            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS room_participants (\n            room_id     INTEGER NOT NULL,\n            user_id     INTEGER NOT NULL,\n            PRIMARY KEY (room_id, user_id),
            FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (\n            id              INTEGER PRIMARY KEY AUTOINCREMENT,\n            room_id         INTEGER NOT NULL,
            sender_username TEXT NOT NULL, -- For simplicity; could be user_id FK
            content         TEXT NOT NULL,\n            timestamp       DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS friend_requests (\n            id              INTEGER PRIMARY KEY AUTOINCREMENT,\n            from_user_id    INTEGER NOT NULL,\n            to_user_id      INTEGER NOT NULL,\n            status          TEXT NOT NULL DEFAULT 'pending', -- pending, accepted, rejected
            timestamp       DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(from_user_id, to_user_id),
            FOREIGN KEY (from_user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (to_user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS auth_tokens (\n            token           TEXT PRIMARY KEY,\n            user_id         INTEGER NOT NULL,\n            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,\n            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    Ok(())
}

// --- Auth Token Functions ---

pub fn create_auth_token(conn: &Connection, user_id: i32) -> Result<String> {
    let token = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO auth_tokens (token, user_id) VALUES (?1, ?2)",
        params![token, user_id],
    )?;
    Ok(token)
}

pub fn get_user_by_token(conn: &Connection, token: &str) -> Result<User> {
    conn.query_row(
        "SELECT u.id, u.username, u.password_hash, u.role \n         FROM users u JOIN auth_tokens t ON u.id = t.user_id \n         WHERE t.token = ?1",
        params![token],
        |row| {
            Ok(User {
                _id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                role: row.get(3)?,
            })
        },
    )
}


// --- User Functions ---

pub fn create_user(conn: &Connection, username: &str, password: &str, role: Option<&str>) -> Result<()> {
    let role = if let Some(r) = role {
        r
    } else {
        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        if user_count == 0 { "admin" } else { "normal" }
    };

    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

    conn.execute(
        "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3)",
        params![username, password_hash, role],
    )?;

    Ok(())
}

pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<User> {
    conn.query_row(
        "SELECT id, username, password_hash, role FROM users WHERE username = ?1",
        params![username],
        |row| {
            Ok(User {
                _id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                role: row.get(3)?,
            })
        },
    )
}

/// Retrieves all users from the database.
pub fn get_all_users(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare("SELECT id, username, password_hash, role FROM users")?;
    let user_iter = stmt.query_map([], |row| {
        Ok(User {
            _id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            role: row.get(3)?,
        })
    })?;
    Ok(user_iter.collect::<Result<Vec<User>>>()?) 
}

/// Deletes a user from the database by their ID.
pub fn delete_user(conn: &Connection, user_id: i32) -> Result<usize> {
    conn.execute("DELETE FROM users WHERE id = ?1", params![user_id])
}

/// Updates a user's role in the database.
pub fn set_user_role(conn: &Connection, username: &str, role: &str) -> Result<usize> {
    conn.execute(
        "UPDATE users SET role = ?1 WHERE username = ?2",
        params![role, username],
    )
}

/// Retrieves all rooms and their participants for the admin panel.
pub fn get_all_rooms(conn: &Connection) -> Result<Vec<AdminRoomInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, is_private, created_at FROM rooms ORDER BY created_at DESC",
    )?;
    let room_iter = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;

    let mut rooms_info = Vec::new();
    for room_result in room_iter {
        let (id, name, is_private, created_at): (i64, Option<String>, bool, String) = room_result?;

        let mut participant_stmt = conn.prepare(
            "SELECT u.username FROM users u JOIN room_participants rp ON u.id = rp.user_id WHERE rp.room_id = ?1",
        )?;
        let participant_iter = participant_stmt.query_map(params![id], |row| row.get(0))?;
        let participants = participant_iter.collect::<Result<Vec<String>>>()?;

        rooms_info.push(AdminRoomInfo {
            id,
            name,
            is_private,
            created_at,
            participants,
        });
    }

    Ok(rooms_info)
}

/// Deletes a room from the database by its ID.
pub fn delete_room(conn: &Connection, room_id: i64) -> Result<usize> {
    conn.execute("DELETE FROM rooms WHERE id = ?1", params![room_id])
}



// --- Message Functions ---

pub fn create_message(conn: &Connection, room_id: i64, sender_username: &str, content: &str) -> Result<ChatMessage> {
    conn.execute(
        "INSERT INTO messages (room_id, sender_username, content) VALUES (?1, ?2, ?3)",
        params![room_id, sender_username, content],
    )?;

    let last_id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, room_id, sender_username, content, timestamp FROM messages WHERE id = ?1",
        params![last_id],
        |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                room_id: row.get(1)?,
                sender_username: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        },
    )
}

pub fn get_messages_for_room(conn: &Connection, room_id: i64) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare("SELECT id, room_id, sender_username, content, timestamp FROM messages WHERE room_id = ?1 ORDER BY timestamp ASC")?;
    let msg_iter = stmt.query_map(params![room_id], |row| {
        Ok(ChatMessage {
            id: row.get(0)?,
            room_id: row.get(1)?,
            sender_username: row.get(2)?,
            content: row.get(3)?,
            timestamp: row.get(4)?,
        })
    })?;
    Ok(msg_iter.collect::<Result<Vec<ChatMessage>>>()?) 
}

// --- Room & Friendship Functions ---

/// Gets all rooms for a given user, including a potential custom name and all participants.
pub fn get_user_rooms(conn: &Connection, user_id: i32) -> Result<Vec<RoomInfo>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.name\n         FROM rooms r\n         JOIN room_participants rp ON r.id = rp.room_id\n         WHERE rp.user_id = ?1 ORDER BY r.created_at DESC"
    )?;
    let room_iter = stmt.query_map(params![user_id], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let mut rooms_info = Vec::new();
    for room_result in room_iter {
        if let Ok((room_id, name)) = room_result {
            let mut p_stmt = conn.prepare(
                "SELECT u.username FROM users u JOIN room_participants rp ON u.id = rp.user_id WHERE rp.room_id = ?1 ORDER BY u.username"
            )?;
            let participants = p_stmt.query_map(params![room_id], |row| row.get(0))?.collect::<Result<Vec<String>>>()?;
            rooms_info.push(RoomInfo { room_id, name, participants });
        }
    }
    Ok(rooms_info)
}

/// Finds a private room between two users, or creates one if it doesn't exist.
pub fn get_or_create_private_room(conn: &mut Connection, user1_id: i32, user2_id: i32) -> Result<i64> {
    let tx = conn.transaction()?;

    let room_id: Option<i64> = tx.query_row(
        "SELECT rp1.room_id\n         FROM room_participants rp1\n         JOIN room_participants rp2 ON rp1.room_id = rp2.room_id\n         JOIN rooms r ON rp1.room_id = r.id\n         WHERE rp1.user_id = ?1 AND rp2.user_id = ?2 AND r.is_private = TRUE",
        params![user1_id, user2_id],
        |row| row.get(0),
    ).optional()?;

    if let Some(id) = room_id {
        tx.commit()?;
        Ok(id)
    } else {
        tx.execute("INSERT INTO rooms (is_private) VALUES (TRUE)", [])?;
        let new_room_id = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO room_participants (room_id, user_id) VALUES (?1, ?2)",
            params![new_room_id, user1_id],
        )?;
        tx.execute(
            "INSERT INTO room_participants (room_id, user_id) VALUES (?1, ?2)",
            params![new_room_id, user2_id],
        )?;

        tx.commit()?;
        Ok(new_room_id)
    }
}


// --- Friend Request Functions ---

pub fn send_friend_request(conn: &Connection, from_user_id: i32, to_user_id: i32) -> Result<FriendRequestInfo> {
    // First, try to insert. The UNIQUE constraint will prevent duplicates.
    conn.execute(
        "INSERT OR IGNORE INTO friend_requests (from_user_id, to_user_id) VALUES (?1, ?2)",
        params![from_user_id, to_user_id],
    )?;

    // Now, whether it was inserted or ignored, fetch the definitive request info.
    conn.query_row(
        "SELECT r.id, r.from_user_id, u.username, r.to_user_id, r.status, r.timestamp\n         FROM friend_requests r\n         JOIN users u ON r.from_user_id = u.id\n         WHERE r.from_user_id = ?1 AND r.to_user_id = ?2",
        params![from_user_id, to_user_id],
        |row| {
            Ok(FriendRequestInfo {
                id: row.get(0)?,
                from_user_id: row.get(1)?,
                from_username: row.get(2)?,
                to_user_id: row.get(3)?,
                status: row.get(4)?,
                timestamp: row.get(5)?,
            })
        },
    )
}

pub fn get_friend_requests(conn: &Connection, user_id: i32) -> Result<Vec<FriendRequestInfo>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.from_user_id, u.username, r.to_user_id, r.status, r.timestamp\n         FROM friend_requests r\n         JOIN users u ON r.from_user_id = u.id\n         WHERE r.to_user_id = ?1 AND r.status = 'pending'"
    )?;
    let req_iter = stmt.query_map(params![user_id], |row| {
        Ok(FriendRequestInfo {
            id: row.get(0)?,
            from_user_id: row.get(1)?,
            from_username: row.get(2)?,
            to_user_id: row.get(3)?,
            status: row.get(4)?,
            timestamp: row.get(5)?,
        })
    })?;
    Ok(req_iter.collect::<Result<Vec<FriendRequestInfo>>>()?) 
}

pub fn accept_friend_request(conn: &mut Connection, request_id: i32) -> Result<Option<i32>> {
    let tx = conn.transaction()?;

    let (from_user_id, to_user_id): (i32, i32) = tx.query_row(
        "SELECT from_user_id, to_user_id FROM friend_requests WHERE id = ?1 AND status = 'pending'",
        params![request_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    tx.execute(
        "UPDATE friend_requests SET status = 'accepted' WHERE id = ?1",
        params![request_id],
    )?;

    // Create a private room for them, which now signifies the friendship.
    let _ = get_or_create_private_room_in_tx(&tx, from_user_id, to_user_id)?;

    tx.commit()?;
    Ok(Some(from_user_id))
}

/// Separate helper to work within an existing transaction
fn get_or_create_private_room_in_tx(tx: &rusqlite::Transaction, user1_id: i32, user2_id: i32) -> Result<i64> {
    let room_id: Option<i64> = tx.query_row(
        "SELECT rp1.room_id\n         FROM room_participants rp1\n         JOIN room_participants rp2 ON rp1.room_id = rp2.room_id\n         JOIN rooms r ON rp1.room_id = r.id\n         WHERE rp1.user_id = ?1 AND rp2.user_id = ?2 AND r.is_private = TRUE",
        params![user1_id, user2_id],
        |row| row.get(0),
    ).optional()?;

    if let Some(id) = room_id {
        Ok(id)
    } else {
        // No room found, create one.
        tx.execute("INSERT INTO rooms (is_private) VALUES (TRUE)", [])?;
        let new_room_id = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO room_participants (room_id, user_id) VALUES (?1, ?2)",
            params![new_room_id, user1_id],
        )?;
        tx.execute(
            "INSERT INTO room_participants (room_id, user_id) VALUES (?1, ?2)",
            params![new_room_id, user2_id],
        )?;

        Ok(new_room_id)
    }
}

pub fn reject_friend_request(conn: &Connection, request_id: i32) -> Result<Option<i32>> {
    let sender_id: Option<i32> = conn.query_row(
        "SELECT from_user_id FROM friend_requests WHERE id = ?1",
        params![request_id],
        |row| row.get(0)
    ).optional()?;

    conn.execute(
        "UPDATE friend_requests SET status = 'rejected' WHERE id = ?1",
        params![request_id],
    )?;
    Ok(sender_id)
}

pub fn get_friends(conn: &Connection, user_id: i32) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT\n            CASE\n                WHEN from_user_id = ?1 THEN to_user_id\n                ELSE from_user_id\n            END AS friend_id\n         FROM friend_requests\n         WHERE (from_user_id = ?1 OR to_user_id = ?1) AND status = 'accepted'"
    )?;
    let friend_ids: Vec<i32> = stmt.query_map(params![user_id], |row| row.get(0))?
                                   .collect::<Result<Vec<i32>>>()?;

    if friend_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Manually construct the IN clause for the query
    let placeholders: Vec<String> = friend_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, username, password_hash, role FROM users WHERE id IN ({})",
        placeholders.join(",")
    );

    // Convert Vec<i32> to Vec<&dyn ToSql> for query_map
    let params_for_query: Vec<&dyn rusqlite::ToSql> = friend_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();

    let mut stmt = conn.prepare(&sql)?;
    let user_iter = stmt.query_map(&*params_for_query, |row| {
        Ok(User {
            _id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            role: row.get(3)?,
        })
    })?;

    user_iter.collect::<Result<Vec<User>>>()
}

pub fn delete_friend(conn: &mut Connection, user1_id: i32, user2_id: i32) -> Result<()> {
    let tx = conn.transaction()?;

    // 1. Find the private room ID between the two users
    let room_id: Option<i64> = tx.query_row(
        "SELECT rp1.room_id\n         FROM room_participants rp1\n         JOIN room_participants rp2 ON rp1.room_id = rp2.room_id\n         JOIN rooms r ON rp1.room_id = r.id\n         WHERE rp1.user_id = ?1 AND rp2.user_id = ?2 AND r.is_private = TRUE",
        params![user1_id, user2_id],
        |row| row.get(0),
    ).optional()?;

    // 2. If a room exists, delete it (cascades to participants and messages)
    if let Some(id) = room_id {
        tx.execute("DELETE FROM rooms WHERE id = ?1", params![id])?;
    }

    // 3. Delete the friend request entry
    tx.execute(
        "DELETE FROM friend_requests 
         WHERE (from_user_id = ?1 AND to_user_id = ?2 AND status = 'accepted') 
            OR (from_user_id = ?2 AND to_user_id = ?1 AND status = 'accepted')",
        params![user1_id, user2_id],
    )?;

    tx.commit()?;
    Ok(())
}
