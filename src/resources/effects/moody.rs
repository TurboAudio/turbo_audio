use crate::resources::color::Color;

#[derive(Debug)]
pub struct Moody {
}

#[derive(Clone, Copy, Debug)]
pub struct MoodySettings {
    pub color: Color,
}

pub fn update_moody(leds: &mut [Color], settings: &MoodySettings) {
    for led in leds {
        *led = settings.color;
    }
}
