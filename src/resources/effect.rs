use super::{
    color::Color,
    settings::{MoodySettings, RaindropSettings},
};
use rand::Rng;

pub struct Moody {
    pub id: i32,
    pub settings_id: i32,
}

pub struct Raindrops {
    pub id: i32,
    pub settings_id: i32,
    pub state: RaindropState,
}
#[derive(Clone, Copy)]
pub enum RipleDirection {
    Left,
    Right,
}
pub struct RaindropState {
    pub riples: Vec<(usize, Color, RipleDirection)>,
}

pub enum Effect {
    Moody(Moody),
    Raindrop(Raindrops),
}

pub fn update_moody(leds: &mut [Color], settings: &MoodySettings) {
    for led in leds {
        *led = settings.color;
    }
}

pub fn update_raindrop(leds: &mut [Color], settings: &RaindropSettings, state: &mut RaindropState) {
    for led in leds.iter_mut() {
        *led = Color::new();
    }

    let color_size = leds.len();
    let mut next_riples: Vec<(usize, Color, RipleDirection)> = vec![];
    const SHIFT: usize = 1;
    for (current_position, color, direction) in &state.riples {
        let next_position = match direction {
            RipleDirection::Left => {
                if current_position < &SHIFT {
                    continue;
                }
                current_position - SHIFT
            }
            RipleDirection::Right => {
                if current_position + SHIFT >= color_size {
                    continue;
                }
                (current_position + SHIFT) as usize
            }
        };
        let next_color = Color {
            r: color.r / 2,
            g: color.g / 2,
            b: color.b / 2,
        };
        if let Some(led) = leds.get_mut(next_position) {
            led.add(&next_color);
            next_riples.push((next_position, next_color, *direction));
        }
    }
    for _ in 0..settings.rain_speed {
        let new_position = rand::thread_rng().gen_range(0..color_size);
        let next_color = Color {
            r: 255,
            g: 255,
            b: 255,
        };
        *leds.get_mut(new_position).expect("Rng lib failed.") = next_color;
        next_riples.push((new_position, next_color, RipleDirection::Left));
        next_riples.push((new_position, next_color, RipleDirection::Right));
    }

    state.riples = next_riples;
}
