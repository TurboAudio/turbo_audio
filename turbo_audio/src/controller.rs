use crate::{
    audio::audio_processing::AudioSignalProcessor,
    hot_reloader::{HotReloader, WatchablePath},
    plugins::effects::{lua::LuaEffectsManager, native::NativeEffectsManager},
    resources::ledstrip::LedStrip,
    Connection, Effect, EffectSettings,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[allow(unused)]
pub struct Controller {
    // settings id to EffectsSettings
    settings: HashMap<usize, EffectSettings>,
    // effect id to Effect. Is an option so that we can drop them first
    effects: Option<HashMap<usize, Effect>>,
    // effect id to settings id.
    effect_settings: HashMap<usize, usize>,

    // connection id to connection
    connections: HashMap<usize, Connection>,
    // led strip id to ledstrip
    led_strips: HashMap<usize, LedStrip>,

    // led strip id to connection id
    led_strip_connections: HashMap<usize, usize>,

    // Effects registry. Effect path to all its instance ids
    effects_registry: HashMap<PathBuf, Vec<usize>>,

    native_effect_manager: NativeEffectsManager,
    lua_effects_manager: LuaEffectsManager,

    hot_reloader: Option<HotReloader>,
}

impl Drop for Controller {
    fn drop(&mut self) {
        // Make sure that the effects are the first things that gets dropped
        self.effects.take();
    }
}

impl Controller {
    pub fn new(audio_processor: &AudioSignalProcessor, lua_package_root: impl AsRef<Path>) -> Self {
        let hot_reloader = HotReloader::new(&[
            WatchablePath::recursive(lua_package_root.as_ref()),
            WatchablePath::recursive(PathBuf::from("../effects/bin").as_ref()),
        ]);

        // Don't propagate the error. Simply log that the hot reloader couldn't be initialized and
        // continue execution
        if let Err(e) = &hot_reloader {
            log::error!("Could not start the effects hot reloader: {e}");
        }

        Self {
            settings: Default::default(),
            effects: Some(Default::default()),
            effect_settings: Default::default(),
            connections: Default::default(),
            led_strips: Default::default(),
            led_strip_connections: Default::default(),
            effects_registry: Default::default(),
            native_effect_manager: NativeEffectsManager::new(audio_processor),
            lua_effects_manager: LuaEffectsManager::new(audio_processor, &lua_package_root),
            hot_reloader: hot_reloader.ok(),
        }
    }

    fn on_file_change(&mut self, path: &Path, effects: &[usize]) {
        let all_lua = effects
            .iter()
            .all(|id| matches!(self.effects.as_ref().unwrap().get(id), Some(Effect::Lua(_))));

        let all_native = effects.iter().all(|id| {
            matches!(
                self.effects.as_ref().unwrap().get(id),
                Some(Effect::Native(_))
            )
        });

        if all_lua {
            self.lua_effects_manager.on_file_changed(path);
        } else if all_native {
            self.native_effect_manager.on_file_changed(path);
        } else {
            log::error!(
                "Not all effects loaded from the file {} are of the same type. This is impossible",
                path.display()
            );
        }
    }

    pub fn check_hot_reload(&mut self) {
        let Some(hot_reloader) = &self.hot_reloader else {
            return;
        };

        let events = hot_reloader.poll_events();

        for event in events {
            let Ok(path) = std::fs::canonicalize(&event.path) else {
                continue;
            };

            let Some(effects) = self.effects_registry.get(&path).map(|x| x.to_owned()) else {
                continue;
            };

            for effect_id in &effects {
                let Some(effect) = self.effects.as_mut().unwrap().get_mut(effect_id) else {
                    continue;
                };

                if let Effect::Native(effect) = effect {
                    self.native_effect_manager.pre_reload_effect(effect);
                };
            }

            self.on_file_change(path.as_ref(), &effects);

            for effect_id in effects {
                let Some(effect) = self.effects.as_mut().unwrap().get_mut(&effect_id) else {
                    continue;
                };

                match effect {
                    Effect::Native(effect) => {
                        self.native_effect_manager.reload_effect(effect);
                    }
                    Effect::Lua(effect) => {
                        self.lua_effects_manager.reload_effect(effect);
                    }
                };
            }
        }
    }

    pub fn add_lua_effect(&mut self, id: usize, effect_path: impl AsRef<Path>) {
        let canonicalized_effect_path = match std::fs::canonicalize(&effect_path) {
            Ok(x) => x,
            Err(e) => {
                log::error!("Couldn't load {}, {e}", effect_path.as_ref().display());
                return;
            }
        };

        let effect = self
            .lua_effects_manager
            .create_effect(&canonicalized_effect_path);

        let effect = match effect {
            Err(e) => {
                log::error!(
                    "Couln't add lua effect: {}. {e:#?}",
                    effect_path.as_ref().display()
                );
                return;
            }
            Ok(x) => x,
        };

        self.on_effect_add(id, canonicalized_effect_path, effect);
    }

    pub fn add_native_effect(&mut self, id: usize, effect_path: impl AsRef<Path>) {
        let Ok(canonicalized_effect_path) = std::fs::canonicalize(&effect_path) else {
            return;
        };

        let effect = self
            .native_effect_manager
            .create_effect(&canonicalized_effect_path);

        let effect = match effect {
            Err(e) => {
                log::error!(
                    "Couln't add native effect: {}. {e:#?}",
                    effect_path.as_ref().display()
                );
                return;
            }
            Ok(x) => x,
        };

        self.on_effect_add(id, canonicalized_effect_path, effect);
    }

    fn on_effect_add(&mut self, id: usize, effect_path: PathBuf, effect: Effect) {
        match self.effects.as_mut().unwrap().entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                log::error!("Couldn't add effect with id {id} because it is already occupied");
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(effect);
            }
        }

        self.effects_registry
            .entry(effect_path)
            .or_default()
            .push(id);
    }

    pub fn add_settings(&mut self, id: usize, settings: EffectSettings) {
        self.settings.insert(id, settings);
    }

    pub fn link_effect_to_settings(&mut self, effect_id: usize, settings_id: usize) -> bool {
        if self.settings.contains_key(&settings_id) {
            self.effect_settings.insert(effect_id, settings_id);
            true
        } else {
            false
        }
    }

    pub fn add_connection(&mut self, connection_id: usize, connection: Connection) {
        self.connections.insert(connection_id, connection);
    }

    pub fn add_led_strip(&mut self, led_strip_id: usize, led_strip: LedStrip) {
        self.led_strips.insert(led_strip_id, led_strip);
    }

    pub fn link_led_strip_to_connection(
        &mut self,
        led_strip_id: usize,
        connection_id: usize,
    ) -> bool {
        if self.connections.contains_key(&connection_id) {
            self.led_strip_connections
                .insert(led_strip_id, connection_id);
            true
        } else {
            false
        }
    }

    pub fn update_led_strips(&mut self) {
        for (led_strip_id, led_strip) in self.led_strips.iter_mut() {
            for (effect_id, interval) in &led_strip.effects {
                let leds = match led_strip.colors.get_mut(interval.0..=interval.1) {
                    Some(leds) => leds,
                    None => {
                        // TODO fix le probleme
                        log::warn!("Effect {effect_id} has invalid interval ({interval:?}) on ledstrip {led_strip_id} of size {}. Skipping.", led_strip.size);
                        continue;
                    }
                };

                let effect = match self.effects.as_mut().unwrap().get_mut(effect_id) {
                    Some(effect) => effect,
                    None => {
                        // TODO fix le probleme
                        log::warn!("Effect {effect_id} doesn't exist. Skipping.");
                        continue;
                    }
                };

                let setting_id = match self.effect_settings.get(effect_id) {
                    Some(effect) => effect,
                    None => {
                        // TODO fix le probleme
                        log::warn!("Settings for effect {effect_id} doesn't exist. Skipping.");
                        continue;
                    }
                };

                let setting = self.settings.get(setting_id);
                match (effect, setting) {
                    (Effect::Lua(lua), Some(EffectSettings::Lua(settings))) => {
                        if let Err(e) = lua.tick(leds, settings) {
                            log::error!("Error when executing lua function: {:?}", e);
                        }
                    }
                    (Effect::Native(native), Some(EffectSettings::Native(_settings))) => {
                        native.tick(leds).unwrap();
                    }
                    _ => panic!("Effect doesn't match settings"),
                }
            }
        }
    }

    pub fn send_ledstrip_colors(&mut self) {
        self.led_strip_connections
            .retain(|ledstrip_id, connection_id| {
                if let Some(ledstrip) = self.led_strips.get(ledstrip_id) {
                    if let Some(connection) = self.connections.get_mut(connection_id) {
                        let data: &[u8] = bytemuck::cast_slice(ledstrip.colors.as_slice());

                        assert!(data.len() == ledstrip.colors.len() * 3);

                        match connection {
                            Connection::Tcp(tcp_connection) => {
                                // If send failsrust use Path for Pathbuf key, connection is closed.
                                if let Err(error) = tcp_connection.send_data(data.to_vec()) {
                                    log::error!("{:?}", error);
                                    self.connections.remove(connection_id);
                                    return false;
                                }
                            }
                            Connection::Usb(_terminal) => {
                                todo!("Implement Usb connection");
                            }
                        }
                        return true;
                    }
                }
                false
            });
    }
}
