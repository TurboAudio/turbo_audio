use rand::Rng;
use std::sync::Mutex;
use turbo_plugin::{Color, Plugin, VTable};

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
        println!("New!!");
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

    fn load() {
        println!("Loading shared library");
    }

    fn unload() {
        println!("Unloading shared library");
    }
}

impl Drop for Soin {
    fn drop(&mut self) {
        println!("Dropping plugin instance");
    }
}

#[no_mangle]
extern "C" fn _plugin_vtable() -> *const std::ffi::c_void {
    extern "C" fn plugin_create() -> *mut std::ffi::c_void {
        let plugin = Box::new(Soin::new());
        Box::into_raw(plugin) as *mut _
    }

    extern "C" fn plugin_destroy(plugin: *mut std::ffi::c_void) {
        unsafe {
            drop(Box::from_raw(plugin as *mut Soin));
        }
    }

    extern "C" fn name(plugin: *const std::ffi::c_void) -> *const std::ffi::c_char {
        let plugin = unsafe { &*(plugin as *const Soin) };
        plugin.name()
    }

    extern "C" fn tick(
        plugin: *const std::ffi::c_void,
        colors: *mut Color,
        len: std::ffi::c_ulong,
    ) {
        let plugin = unsafe { &*(plugin as *const Soin) };
        let slice = unsafe { std::slice::from_raw_parts_mut(colors, len as _) };
        plugin.tick(slice);
    }

    extern "C" fn load(audio_api: turbo_plugin::AudioApi) {
        turbo_plugin::on_load(audio_api);
        Soin::load();
    }

    extern "C" fn unload() {
        Soin::unload();
    }

    static VTABLE: VTable = VTable {
        plugin_create,
        plugin_destroy,
        name,
        tick,
        load,
        unload,
    };

    &VTABLE as *const VTable as *const _
}
