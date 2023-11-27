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
        for led in leds {
            led.r = 255;
            led.g = 255;
        }
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
