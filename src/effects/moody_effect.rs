use serde::{Serialize, Deserialize};
use crate::Effect;
use crate::core::color::Color;

#[derive(Serialize, Deserialize, Debug)]
pub struct MoodyEffect {
    name: String,
    id: i64,
    settings_id: i64,
    effect_type: String,
    colors: Vec<Color>,
}

impl Effect for MoodyEffect {
    fn new() -> Self {
        Self {
            name: String::from("Moody Effect"),
            id: 0,
            settings_id: 0,
            effect_type: String::from("Moody effect"),
            colors: Vec::new(),
        }
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn update(&self, colors: &mut [Color]) {
        
    }

    fn get_id(&self) -> i64 {
        self.id
    }

    fn get_settings_id(&self) -> i64 {
        self.settings_id
    }

    fn get_colors(&self) -> &[Color] {
        &self.colors[..]
    }

    fn set_number_of_leds(&mut self, size: usize) {

    }
    
    fn get_number_of_leds(&self) -> usize {
        0
    }

    fn serialize(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    fn deserialize(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }

}

impl MoodyEffect {

}
