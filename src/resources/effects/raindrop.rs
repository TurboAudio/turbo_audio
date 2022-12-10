use std::ops::{Mul, Div};

use crate::resources::color::Color;
use rand::Rng;

#[derive(Clone, Copy)]
pub struct RaindropSettings {
    pub rain_speed: i32,
    pub drop_rate: f64,
}

pub struct Raindrops {
    pub id: i32,
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
        const NUMERATOR: u8 = 3;
        const DENOMINATOR: u8 = 4;
        let next_color = Color {
            r: color.r.div(DENOMINATOR).mul(NUMERATOR),
            g: color.g.div(DENOMINATOR).mul(NUMERATOR),
            b: color.b.div(DENOMINATOR).mul(NUMERATOR),
        };
        if let Some(led) = leds.get_mut(next_position) {
            led.add(&next_color);
            next_riples.push((next_position, next_color, *direction));
        }
    }

    if !rand::thread_rng().gen_bool(settings.drop_rate) {
        state.riples = next_riples;
        return;
    }

    for _ in 0..settings.rain_speed {
        let new_position = rand::thread_rng().gen_range(0..color_size);
        let next_color = match rand::thread_rng().gen_range(0..4) {
            0 => Color {r: 255, g: 0, b: 255},
            1 => Color {r: 255, g: 255, b: 0},
            2 => Color {r: 0, g: 255, b: 255},
            3 => Color {r: 255, g: 255, b: 255},
            _ => unreachable!()
        };
        next_riples.push((new_position, next_color, RipleDirection::Left));
        next_riples.push((new_position, next_color, RipleDirection::Right));
    }

    state.riples = next_riples;
}
