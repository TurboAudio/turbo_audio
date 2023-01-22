use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex, RwLock,
    },
    thread::JoinHandle,
};

use crate::{
    audio_processing::AudioSignalProcessor,
    hot_reload::start_hot_reload_lua_effects,
    resources::{effects::lua::LuaEffect, settings::Settings},
};

type EffectSettings = RwLock<HashMap<usize, Settings>>;

pub enum ServerEvent {
    NewLuaEffect(String, LuaEffect),
}
#[derive(Default)]
pub struct ServerState {
    pub settings: EffectSettings,
}

pub struct Server {
    state: Arc<ServerState>,
    server_thread: Option<JoinHandle<()>>,
    force_exit_server: Arc<Mutex<bool>>,
    audio_processor: Arc<RwLock<AudioSignalProcessor>>,
}

impl Server {
    pub fn new(audio_processor: Arc<RwLock<AudioSignalProcessor>>) -> Self {
        Self {
            state: Arc::default(),
            server_thread: None,
            force_exit_server: Arc::new(Mutex::new(false)),
            audio_processor,
        }
    }

    pub fn start(&mut self) -> Receiver<ServerEvent> {
        let (tx, rx) = mpsc::channel();
        let _state = self.state.clone();
        let should_force_exit = self.force_exit_server.clone();
        let audio_processor = self.audio_processor.clone();
        self.server_thread = Some(std::thread::spawn(move || {
            let (hot_reload_rx, _debouncer) = start_hot_reload_lua_effects().unwrap();
            loop {
                if let Ok(should_exit) = should_force_exit.lock() {
                    if *should_exit {
                        log::trace!("Exiting server");
                        break;
                    }
                }

                if let Ok(Ok(events)) = hot_reload_rx.try_recv() {
                    for event in &events {
                        if let Some(filename) = event.path.to_str() {
                            if let Some(start) = filename.find("/./") {
                                let filename = &filename[start + 3..];
                                if let Ok(lua_effect) =
                                    LuaEffect::new(filename, audio_processor.clone())
                                {
                                    let _ = tx.send(ServerEvent::NewLuaEffect(
                                        filename.to_string(),
                                        lua_effect,
                                    ));
                                }
                                log::info!("Reloaded effect {filename}");
                            }
                        }
                    }
                }
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
