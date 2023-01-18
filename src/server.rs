use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{
    hyper::Response,
    ws::{Message, WebSocket},
    Filter,
};

use crate::{resources::{effects::Effect, settings::Settings}, connections::Connection};
type IdCounter = Arc<AtomicUsize>;
type EffectMap = Arc<RwLock<HashMap<usize, Effect>>>;
type SettingsMap = Arc<RwLock<HashMap<usize, Settings>>>;
type ConnectionMap = Arc<RwLock<HashMap<usize, Connection>>>;
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[derive(Default, Clone)]
pub struct ServerState {
    pub id_counter: IdCounter,
    pub effects: EffectMap,
    pub settings: SettingsMap,
    pub connections: ConnectionMap,
}


#[derive(Debug, Deserialize, Serialize)]
struct ResourceRequest {
    resource_type: String,
    value: i32,
}


pub struct WebSocketServer {
    users: Users,
    state: ServerState,
    server_handle: Option<JoinHandle<()>>,
}

impl WebSocketServer {
    pub fn new(state: ServerState) -> WebSocketServer {
        WebSocketServer {
            users: Users::default(),
            state,
            server_handle: None,
        }
    }

    pub fn start_server(&mut self, handle: tokio::runtime::Handle) {
        let users = self.users.clone();
        let id_counter = self.state.id_counter.clone();
        let websocket_route = warp::path("ws")
            .and(warp::ws())
            .and(warp::any().map(move || id_counter.clone()))
            .and(warp::any().map(move || users.clone()))
            .map(
                |ws: warp::ws::Ws, user_id_counter: IdCounter, users: Users| {
                    ws.on_upgrade(move |socket| {
                        WebSocketServer::handle_connection(user_id_counter, socket, users)
                    })
                },
            );

        // let effect_get = warp::get()
        //     .and(warp::path("effects"))
        //     .and(warp::path::param::<usize>())
        //     .and(warp::any().map(move || self.state.effects.clone()))
        //     .and_then(|effect_id, effects: EffectMap| async move {
        //         match effects.read().await.get(&effect_id) {
        //             Some(value) => {
        //                 let body = match value {
        //                     Effect::Lua(lua_effect) => Ok("Lua"),
        //                     Effect::Moody(moody) => Ok("Moody"),
        //                     Effect::Raindrop(raindrop) => Ok("Raindrop")
        //                 };
        //                 Ok(Response::builder()
        //                 .body(body)
        //                 .unwrap())
        //             },
        //             None => Err(warp::reject::not_found()),
        //         }
        //     });

        // let effect_post = warp::post()
        //     .and(warp::path("effects"))
        //     .and(warp::body::json())
        //     .and(warp::any().map(move || resources.clone()))
        //     .and(warp::any().map(move || resources_id_counter.clone()))
        //     .and_then(
        //         |request: ResourceRequest, resources: ResourceMap, id_counter: IdCounter| {
        //             async move {
        //                 let new_resource_id = id_counter.fetch_add(1, Ordering::Relaxed);
        //                 let new_resource = match &request.resource_type[..] {
        //                     "Setting" => Resource::Setting {
        //                         id: new_resource_id,
        //                         value: request.value,
        //                     },
        //                     "Effect" => Resource::Setting {
        //                         id: new_resource_id,
        //                         value: request.value,
        //                     },
        //                     _ => return Err(warp::reject::not_found()),
        //                 };
        //
        //                 resources.write().await.insert(new_resource_id, new_resource.clone());
        //                 Ok(Response::builder().body(serde_json::to_string(&new_resource).unwrap()).unwrap())
        //             }
        //         },
        //     );
        let api = websocket_route;
        self.server_handle = Some(handle.spawn(warp::serve(api).run(([127, 0, 0, 1], 9001))));
    }

    pub fn close(self) {
        match self.server_handle {
            Some(handle) => handle.abort(),
            None => println!("No server handle"),
        };
    }

    async fn handle_connection(user_id: IdCounter, ws: WebSocket, users: Users) {
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
