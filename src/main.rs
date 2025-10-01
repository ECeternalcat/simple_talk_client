use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod db;
mod handler;

// --- Type Aliases for Clarity ---
pub type RoomId = i64;

// --- Core Application Structs ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config { port: 3001 }
    }
}

fn load_config() -> Config {
    let path = Path::new("config.json");
    if !path.exists() {
        let config = Config::default();
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            fs::write(path, json).expect("Failed to write default config file.");
        }
        return config;
    }

    let file_content = fs::read_to_string(path).expect("Failed to read config file.");
    serde_json::from_str(&file_content).unwrap_or_else(|e| {
        tracing::error!("Failed to parse config.json: {}. Using default config.", e);
        Config::default()
    })
}

#[derive(Default)]
pub struct Room {
    pub clients: HashMap<i32, Client>, // Keyed by user_id
}

pub struct Client {
    pub sender: mpsc::UnboundedSender<Message>,
}

pub struct AppState {
    pub rooms: Mutex<HashMap<RoomId, Room>>,
    pub online_users: Mutex<HashMap<i32, mpsc::UnboundedSender<Message>>>, // user_id -> sender
    pub db_pool: db::Pool,
    pub shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>
}

// --- WebSocket Message Structures ---

#[derive(Deserialize, Debug)]
pub struct WsRequestMessage {
    pub r#type: String,
    pub payload: serde_json::Value,
}

#[derive(Serialize)]
pub struct WsResponseMessage {
    pub r#type: String,
    pub payload: serde_json::Value,
}

// Payloads for specific message types

#[derive(Deserialize, Debug)]
pub struct RegisterPayload {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthWithTokenPayload {
    pub token: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JoinRoomPayload {
    pub room_id: RoomId,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessagePayload {
    pub room_id: RoomId,
    pub content: String,
}

#[derive(Deserialize, Debug)]
pub struct SendFriendRequestPayload {
    pub username: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RespondToFriendRequestPayload {
    pub request_id: i32,
    pub accept: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuickChatPayload {
    pub friend_id: i32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFriendPayload {
    pub friend_id: i32,
}

#[derive(Serialize, Clone)]
pub struct InvitationPayload {
    pub from_username: String,
    pub room_id: RoomId,
}

#[derive(Serialize, Clone)]
pub struct VoiceChatInvitationPayload {
    pub from_username: String,
}

#[derive(Deserialize, Debug)]
pub struct AdminCreateUserPayload {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Deserialize, Debug)]
pub struct AdminDeleteUserPayload {
    pub user_id: i32,
}

#[derive(Deserialize, Debug)]
pub struct AdminDeleteRoomPayload {
    pub room_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct AdminChangePortPayload {
    pub port: u16,
}

#[derive(Serialize)]
pub struct FriendInfo {
    pub id: i32,
    pub username: String,
    pub is_online: bool,
}

// --- Main Application Logic ---

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    db::init_db()?;
    println!("Database initialized successfully.");

    let config = load_config();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let shared_state = Arc::new(AppState {
        rooms: Mutex::new(HashMap::new()),
        online_users: Mutex::new(HashMap::new()),
        db_pool: db::DB_POOL.clone(),
        shutdown_tx: Mutex::new(Some(shutdown_tx)),
    });
    let app = Router::new()
        .nest_service("/", ServeDir::new("public"))
        .route("/ws", get(ws_handler))
        .with_state(shared_state.clone());

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let actual_port = listener.local_addr()?.port();
    println!("\n--- Server Started ---");
    println!("  > Listening on: {}", addr);
    println!("  > On this machine: http://localhost:{}", actual_port);
    if let Ok(ifaces) = get_if_addrs::get_if_addrs() {
        for iface in ifaces {
            if !iface.is_loopback() {
                println!("  > On local network: http://{}:{}", iface.ip(), actual_port);
            }
        }
    }
    println!("----------------------\n");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>()
    )
    .with_graceful_shutdown(async {
        shutdown_rx.await.ok();
        println!("Graceful shutdown initiated...");
    })
    .await?;

    println!("Server has shut down.");
    Ok(())
}

// --- WebSocket Handling ---

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let user = match authenticate(&mut ws_sender, &mut ws_receiver, &state).await {
        Some(user) => user,
        None => {
            tracing::warn!("Client failed authentication or disconnected during auth.");
            return;
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    state.online_users.lock().unwrap().insert(user._id, tx.clone());
    tracing::info!("User '{}' (id: {}) connected.", user.username, user._id);

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let recv_state = state.clone();
    let user_clone_for_cleanup = user.clone();
    let mut users_current_room_id: Option<RoomId> = None;

    let mut recv_task = tokio::spawn(async move {
        handler::handle_get_user_rooms(recv_state.clone(), &user, &tx).await;
        handler::handle_get_friend_requests(recv_state.clone(), &user, &tx).await;
        handler::handle_get_friend_list(recv_state.clone(), &user, &tx).await;

        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(req) = serde_json::from_str::<WsRequestMessage>(&text) {
                        handler::handle_message(req, recv_state.clone(), &user, &mut users_current_room_id, &tx).await;
                    } else {
                        tracing::warn!("Failed to parse incoming message: {}", text);
                    }
                }
                Message::Binary(data) => {
                    if let Some(ref room_id) = users_current_room_id {
                        let rooms = recv_state.rooms.lock().unwrap();
                        if let Some(room) = rooms.get(room_id) {
                            for (peer_id, client) in room.clients.iter() {
                                if *peer_id != user._id {
                                    let _ = client.sender.send(Message::Binary(data.clone()));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        users_current_room_id
    });

    let final_room_id = tokio::select! {
        room_id = (&mut recv_task) => room_id.ok().flatten(),
        _ = (&mut send_task) => None,
    };

    state.online_users.lock().unwrap().remove(&user_clone_for_cleanup._id);
    tracing::info!("User '{}' disconnected.", user_clone_for_cleanup.username);

    if let Some(room_id) = final_room_id {
        let mut rooms = state.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&room_id) {
            if room.clients.remove(&user_clone_for_cleanup._id).is_some() {
                tracing::info!("Removed user from room '{}' in memory.", room_id);
            }
            if room.clients.is_empty() {
                rooms.remove(&room_id);
                tracing::info!("Room '{}' is now empty and has been removed from memory.", room_id);
            }
        }
    }
}

async fn authenticate(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    receiver: &mut futures_util::stream::SplitStream<WebSocket>,
    state: &Arc<AppState>,
) -> Option<db::User> {
    loop {
        if let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(msg) = serde_json::from_str::<WsRequestMessage>(&text) {
                match msg.r#type.as_str() {
                    "register" => {
                        if let Ok(p) = serde_json::from_value::<RegisterPayload>(msg.payload) {
                            let conn = state.db_pool.get().unwrap();
                            match db::create_user(&conn, &p.username, &p.password, None) {
                                Ok(_) => {
                                    send_ws_message(sender, "register_ok", "Registration successful. Please log in.").await;
                                }
                                Err(e) => {
                                    send_ws_message(sender, "register_fail", e.to_string()).await;
                                }
                            }
                        }
                        return None; // Close connection after any registration attempt
                    }
                    "login" => {
                        if let Ok(p) = serde_json::from_value::<LoginPayload>(msg.payload) {
                            let conn = state.db_pool.get().unwrap();
                            if let Ok(user) = db::get_user_by_username(&conn, &p.username) {
                                if bcrypt::verify(&p.password, &user.password_hash).unwrap_or(false) {
                                    match db::create_auth_token(&conn, user._id) {
                                        Ok(token) => {
                                            let payload = serde_json::json!({
                                                "username": user.username.clone(),
                                                "role": user.role.clone(),
                                                "token": token
                                            });
                                            send_ws_message(sender, "auth_ok", payload).await;
                                            return Some(user);
                                        }
                                        Err(_) => {
                                            send_ws_message(sender, "auth_fail", "Failed to create auth token.").await;
                                        }
                                    }
                                } else {
                                    send_ws_message(sender, "auth_fail", "Invalid username or password.").await;
                                }
                            } else {
                                send_ws_message(sender, "auth_fail", "Invalid username or password.").await;
                            }
                        }
                        return None; // Close connection on any failed login path
                    }
                    "auth_with_token" => {
                        if let Ok(p) = serde_json::from_value::<AuthWithTokenPayload>(msg.payload) {
                            let conn = state.db_pool.get().unwrap();
                            if let Ok(user) = db::get_user_by_token(&conn, &p.token) {
                                let payload = serde_json::json!({
                                    "username": user.username.clone(),
                                    "role": user.role.clone(),
                                    "token": p.token
                                });
                                send_ws_message(sender, "auth_ok", payload).await;
                                return Some(user);
                            }
                        }
                        // If token auth fails, just close the connection.
                        return None;
                    }
                    _ => return None,
                }
            }
        }
    }
}

pub async fn send_ws_message<T: Serialize>(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    r#type: &str,
    payload: T,
) {
    let resp = WsResponseMessage {
        r#type: r#type.to_string(),
        payload: serde_json::to_value(payload).unwrap(),
    };
    if sender.send(Message::Text(serde_json::to_string(&resp).unwrap())).await.is_err() {
        tracing::warn!("Failed to send direct message to client.");
    }
}

pub async fn send_ws_message_to<T: Serialize>(
    sender: &mpsc::UnboundedSender<Message>,
    r#type: &str,
    payload: T,
) {
    let resp = WsResponseMessage {
        r#type: r#type.to_string(),
        payload: serde_json::to_value(payload).unwrap(),
    };
    if sender.send(Message::Text(serde_json::to_string(&resp).unwrap())).is_err() {
        tracing::warn!("Failed to send channel message to client.");
    }
}
