use rand::Rng;
use std::sync::Mutex;
use turbo_plugin::{make_plugin, Color, Plugin, VTable};

#[derive(Clone, Copy, Debug)]
pub struct RaindropSettings {
    pub rain_speed: i32,
    pub drop_rate: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum RipleDirection {
    Left,
    Right,
}

#[derive(Debug, Default)]
pub struct RaindropState {
    riples: Vec<(usize, Color, RipleDirection)>,
}

struct Soin {
    state: Mutex<RaindropState>,
}

impl Soin {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
        }
    }
}

impl Plugin for Soin {
    fn name(&self) -> *const std::ffi::c_char {
        static NAME: &[u8] = b"Soin\0";
        static CSTR_NAME: &std::ffi::CStr =
            unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(NAME) };
        CSTR_NAME.as_ptr()
    }

    fn tick(&self, leds: &mut [Color]) {
        let mut state = self.state.lock().unwrap();
        leds.fill(Color { r: 0, g: 0, b: 0 });
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
                    current_position + SHIFT
                }
            };
            const NUMERATOR: u8 = 3;
            const DENOMINATOR: u8 = 4;
            let next_color = Color {
                r: color.r / DENOMINATOR * NUMERATOR,
                g: color.g / DENOMINATOR * NUMERATOR,
                b: color.b / DENOMINATOR * NUMERATOR,
            };
            if let Some(led) = leds.get_mut(next_position) {
                led.r += next_color.r;
                led.g += next_color.g;
                led.b += next_color.b;
                next_riples.push((next_position, next_color, *direction));
            }
        }

        if !rand::thread_rng().gen_bool(0.5) {
            state.riples = next_riples;
            return;
        }

        for _ in 0..1 {
            let new_position = rand::thread_rng().gen_range(0..color_size);
            let next_color = match rand::thread_rng().gen_range(0..4) {
                0 => Color {
                    r: 255,
                    g: 0,
                    b: 255,
                },
                1 => Color {
                    r: 255,
                    g: 255,
                    b: 0,
                },
                2 => Color {
                    r: 0,
                    g: 255,
                    b: 255,
                },
                3 => Color {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                _ => unreachable!(),
            };
            *leds.get_mut(new_position).expect("Rng lib failed.") = next_color;
            next_riples.push((new_position, next_color, RipleDirection::Left));
            next_riples.push((new_position, next_color, RipleDirection::Right));
        }

        state.riples = next_riples;
    }

    fn load() {}

    fn unload() {}
}

make_plugin!(Soin, Soin::new());
