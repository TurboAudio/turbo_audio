use std::collections::HashSet;

use super::color::Color;
pub type EffectInterval = (usize, usize);
#[derive(Debug, Default)]
pub struct LedStrip {
    pub size: usize,
    pub colors: Vec<Color>,
    pub effects: Vec<(i32, EffectInterval)>,
    pub connection_id: Option<i32>,
    used_led_count: usize,
}

impl LedStrip {
    pub fn set_led_count(&mut self, size: usize) {
        self.size = size;
        let mut to_remove = HashSet::new();
        for (effect_id, interval) in &self.effects {
            if interval.1 >= size {
                to_remove.insert(*effect_id);
            }
        }
        self.effects
            .retain(|(effect_id, _interval)| !to_remove.contains(effect_id));
        self.colors.resize(size, Color::new());
    }

    pub fn add_effect(&mut self, effect_id: i32, size: usize) -> bool {
        if self.used_led_count + size > self.size {
            return false;
        }

        let interval = (self.used_led_count, self.used_led_count + size - 1);
        self.effects.push((effect_id, interval));
        self.used_led_count += size;
        true
    }
}
