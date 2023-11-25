use crate::{
    effects::native::NativeEffectManager, resources::ledstrip::LedStrip, Connection, Effect,
    EffectSettings,
};
use std::{collections::HashMap, path::PathBuf};

#[derive(Default)]
#[allow(unused)]
pub struct Controller {
    settings: HashMap<usize, EffectSettings>,
    pub effects: Option<HashMap<usize, Effect>>,
    effect_settings: HashMap<usize, usize>,
    connections: HashMap<usize, Connection>,
    led_strips: HashMap<usize, LedStrip>,
    led_strip_connections: HashMap<usize, usize>,
    pub lua_effects_registry: HashMap<PathBuf, Vec<usize>>,
    pub native_effect_manager: NativeEffectManager,
}

impl Drop for Controller {
    fn drop(&mut self) {
        log::info!("Dropping controller");
        self.effects.take();
    }
}

impl Controller {
    pub fn new() -> Self {
        Self {
            settings: Default::default(),
            effects: Some(Default::default()),
            effect_settings: Default::default(),
            connections: Default::default(),
            led_strips: Default::default(),
            led_strip_connections: Default::default(),
            lua_effects_registry: Default::default(),
            native_effect_manager: Default::default(),
        }
    }

    pub fn add_effect(&mut self, id: usize, effect: Effect) {
        if let Effect::Lua(lua_effect) = &effect {
            log::info!("Added lua effect to registry: (id: {id})");
            let lua_file_name = std::fs::canonicalize(lua_effect.get_path()).unwrap();
            match self.lua_effects_registry.get_mut(&lua_file_name) {
                Some(lua_effects) => lua_effects.push(id),
                None => {
                    self.lua_effects_registry.insert(lua_file_name, vec![id]);
                }
            }
        }
        self.effects.as_mut().unwrap().insert(id, effect);
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
                        native.tick().unwrap();
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
