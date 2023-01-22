use std::{sync::{RwLock, Arc, Mutex, mpsc::{Receiver, self}}, collections::HashMap, thread::JoinHandle};

use crate::resources::{settings::Settings, effects::{Effect, moody::Moody}};


type EffectSettings = RwLock<HashMap<usize, Settings>>;

pub enum ServerEvent {
    NewEffect(usize, Effect),
    Pipi(),
}
#[derive(Default)]
pub struct ServerState {
    pub settings: EffectSettings,
}

pub struct Server {
    state: Arc<ServerState>,
    server_thread: Option<JoinHandle<()>>,
    force_exit_server: Arc<Mutex<bool>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            state: Arc::default(),
            server_thread: None,
            force_exit_server: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&mut self) -> Receiver<ServerEvent> {
        let (tx, rx) = mpsc::channel();
        let _state = self.state.clone();
        let should_force_exit = self.force_exit_server.clone();
        let mut alternate = true;
        let mut i: usize = 0;
        self.server_thread = Some(std::thread::spawn(move || {
            loop {
                if let Ok(should_exit) = should_force_exit.lock() {
                    if *should_exit {
                        log::trace!("Exiting server");
                        break;
                    }
                }

                let event = {
                    if alternate {
                        ServerEvent::Pipi()
                    } else {
                        i += 1;
                        ServerEvent::NewEffect(i, Effect::Moody(Moody { id: i as i32}))
                    }
                };

                if tx.send(event).is_err() {
                    break;
                }
                alternate = !alternate;
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }));
        log::trace!("Server stated");
        rx
    }

    pub fn stop(&mut self) {
        if let Ok(mut force_exit) = self.force_exit_server.lock() {
            *force_exit = true;
        }

        if let Some(server_thread) = std::mem::replace(&mut self.server_thread, None) {
            let _ = server_thread.join();
        }
    }
}
