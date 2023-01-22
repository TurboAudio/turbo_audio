use crate::{
    resources::{
        effects::{moody::update_moody, raindrop::update_raindrop},
        ledstrip::LedStrip,
    },
    Connection, Effect, Settings,
};
use std::collections::HashMap;

#[derive(Default)]
#[allow(unused)]
pub struct Controller {
    settings: HashMap<usize, Settings>,
    effects: HashMap<usize, Effect>,
    effect_settings: HashMap<usize, usize>,
    connections: HashMap<usize, Connection>,
    led_strips: HashMap<usize, LedStrip>,
    led_strip_connections: HashMap<usize, usize>,
    lua_effects_registry: HashMap<String, usize>,
}

impl Controller {
    pub fn new() -> Self {
        Controller::default()
    }

    pub fn add_effect(&mut self, id: usize, effect: Effect) {
        if let Effect::Lua(lua_effect) = &effect {
            self.lua_effects_registry
                .insert(lua_effect.get_filename().to_owned(), id);
        }
        self.effects.insert(id, effect);
    }

    pub fn add_settings(&mut self, id: usize, settings: Settings) {
        self.settings.insert(id, settings);
    }

    pub fn link_effect_to_settings(&mut self, effect_id: usize, settings_id: usize) {
        self.effect_settings.insert(effect_id, settings_id);
    }

    pub fn add_connection(&mut self, connection_id: usize, connection: Connection) {
        self.connections.insert(connection_id, connection);
    }

    pub fn add_led_strip(&mut self, led_strip_id: usize, led_strip: LedStrip) {
        self.led_strips.insert(led_strip_id, led_strip);
    }

    pub fn link_led_strip_to_connection(&mut self, led_strip_id: usize, connection_id: usize) {
        self.led_strip_connections
            .insert(led_strip_id, connection_id);
    }

    pub fn update_led_strips(&mut self) {
        for (led_strip_id, led_strip) in &self.led_strips {
            for (effect_id, interval) in &led_strip.effects {
                let leds = match led_strip.colors.get_mut(interval.0..=interval.1) {
                    Some(leds) => leds,
                    None => {
                        // TODO fix le probleme
                        log::warn!("Effect {effect_id} has invalid interval ({interval:?}) on ledstrip {led_strip_id} of size {}. Skipping.", led_strip.size);
                        continue;
                    }
                };

                let effect = match self.effects.get_mut(effect_id) {
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
                    (Effect::Moody(_moody), Some(Settings::Moody(settings))) => {
                        update_moody(leds, &settings);
                    }
                    (Effect::Raindrop(raindrop), Some(Settings::Raindrop(settings))) => {
                        update_raindrop(leds, &settings, &mut raindrop.state);
                    }
                    (Effect::Lua(lua), Some(Settings::Lua(settings))) => {
                        if let Err(e) = lua.tick(leds, &settings) {
                            log::error!("Error when executing lua function: {:?}", e);
                        }
                    }
                    _ => panic!("Effect doesn't match settings"),
                }
            }
        }
    }
}
