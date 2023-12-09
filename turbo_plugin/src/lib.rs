use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    process::abort,
    sync::{Mutex, OnceLock},
};

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable, Deserialize, Serialize)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub trait Plugin: Any + Send + Sync {
    /// Get a name describing the `Plugin`.
    fn name(&self) -> *const std::ffi::c_char;

    /// Tick fn
    fn tick(&self, leds: &mut [Color]);

    /// A callback fired immediately after the plugin is loaded. Usually used
    /// for initialization.
    fn load();

    /// A callback fired immediately before the plugin is unloaded. Use this if
    /// you need to do any cleanup.
    fn unload();
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct VTable {
    /// Function that returns a pointer to a heap allocated plugin
    pub plugin_create: extern "C" fn() -> *mut std::ffi::c_void,

    /// Function that destroys a heap allocated plugin
    pub plugin_destroy: extern "C" fn(*mut std::ffi::c_void),

    /// Function that returns the name of the plugin
    pub name: extern "C" fn(*const std::ffi::c_void) -> *const std::ffi::c_char,

    /// Function that ticks the plugin
    pub tick: extern "C" fn(*const std::ffi::c_void, *mut Color, std::ffi::c_ulong),

    /// Function that gets called when the shared library gets loaded
    /// Useful for making initialization that is shared between plugin instances
    pub load: extern "C" fn(AudioApi),

    /// Function that gets called when the shared library gets unloaded
    /// Useful for cleaning up anything that was initialized during the `on_load` function
    pub unload: extern "C" fn(),
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct AudioApi {
    pub instance: *const std::ffi::c_void,
    pub get_average_amplitude: extern "C" fn(
        *const std::ffi::c_void,
        std::ffi::c_float,
        std::ffi::c_float,
    ) -> std::ffi::c_float,
    pub get_frequency_amplitude:
        extern "C" fn(*const std::ffi::c_void, std::ffi::c_float) -> std::ffi::c_float,
    pub get_max_frequency: extern "C" fn(*const std::ffi::c_void) -> std::ffi::c_float,
}

unsafe impl Send for AudioApi {}
unsafe impl Sync for AudioApi {}

static API_INSTANCE: OnceLock<Mutex<AudioApi>> = OnceLock::new();

pub fn on_load(audio_api: AudioApi) {
    let mut api = API_INSTANCE
        .get_or_init(|| Mutex::new(audio_api))
        .lock()
        .unwrap();
    *api = audio_api;
}

pub fn get_average_amplitude(lower_freq: f32, upper_freq: f32) -> f32 {
    let Some(api) = API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();
    (api.get_average_amplitude)(api.instance, lower_freq, upper_freq)
}

pub fn get_frequency_amplitude(frequency: f32) -> f32 {
    let Some(api) = API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_frequency_amplitude)(api.instance, frequency)
}

pub fn get_max_frequency() -> std::ffi::c_float {
    let Some(api) = API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_max_frequency)(api.instance)
}
