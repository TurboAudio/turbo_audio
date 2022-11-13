use crate::resources::color::Color;

pub struct Moody {
    pub id: i32,
    pub settings_id: i32,
}

#[derive(Clone, Copy)]
pub struct MoodySettings {
    pub color: Color,
}

pub fn update_moody(leds: &mut [Color], settings: &MoodySettings) {
    for led in leds {
        *led = settings.color;
    }
}
