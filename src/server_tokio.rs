use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;
type UserIdCounter = Arc<AtomicUsize>;

pub struct WebSocketServer {
    users: Users,
    server_handle: Option<JoinHandle<()>>,
}

impl WebSocketServer {
    pub fn new() -> WebSocketServer {
        WebSocketServer {
            users: Users::default(),
            server_handle: None,
        }
    }

    pub fn start_server(&mut self, handle: tokio::runtime::Handle) {
        let users = self.users.clone();
        let user_id_counter = UserIdCounter::default();
        let routes = warp::path("echo")
            .and(warp::ws())
            .and(warp::any().map(move || users.clone()))
            .and(warp::any().map(move || user_id_counter.clone()))
            .map(
                |ws: warp::ws::Ws, users: Users, user_id_counter: UserIdCounter| {
                    ws.on_upgrade(move |socket| {
                        WebSocketServer::handle_connection(user_id_counter, socket, users)
                    })
                },
            );

        self.server_handle = Some(handle.spawn(warp::serve(routes).run(([127, 0, 0, 1], 9001))));
    }

    pub fn close(self) {
        match self.server_handle {
            Some(handle) => handle.abort(),
            None => println!("No server handle"),
        };
    }

    async fn handle_connection(user_id: UserIdCounter, ws: WebSocket, users: Users) {
        let (mut user_ws_tx, mut user_ws_rx) = ws.split();
        let (tx, rx) = mpsc::unbounded_channel();
        let mut rx = UnboundedReceiverStream::new(rx);
        tokio::task::spawn(async move {
            while let Some(message) = rx.next().await {
                user_ws_tx
                    .send(message)
                    .unwrap_or_else(|e| {
                        eprintln!("Websocket send error: {}", e);
                    })
                    .await;
            }
        });

        let user_id = user_id.fetch_add(1, Ordering::Relaxed);
        users.write().await.insert(user_id, tx);
        tokio::task::spawn(async move {
            while let Some(result) = user_ws_rx.next().await {
                let msg = match result {
                    Ok(msg) => msg,
                    Err(e) => {
                        println!("websocket error(uid={}): {}", user_id, e);
                        break;
                    }
                };
                WebSocketServer::handle_message(user_id, msg, &users).await;
            }

            WebSocketServer::handle_user_disconnect(user_id, &users).await;
        });
    }

    async fn handle_message(user_id: usize, message: Message, users: &Users) {
        let message = if let Ok(str) = message.to_str() {
            str
        } else {
            return;
        };

        let new_message = format!("<User#{}>: {}", user_id, message);
        for (&uid, tx) in users.read().await.iter() {
            if uid == user_id {
                // Don't echo back message to sender
                continue;
            }

            // If an error occurs, the user is disconnected and will be handled in
            // another task
            if let Err(_disconnected) = tx.send(Message::text(new_message.clone())) {}
        }
    }

    async fn handle_user_disconnect(user_id: usize, users: &Users) {
        println!("User {} disconnected", user_id);
        users.write().await.remove(&user_id);
    }
}
