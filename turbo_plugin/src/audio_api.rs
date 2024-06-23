use std::{
    process::abort,
    sync::{Mutex, OnceLock},
};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct AudioApi {
    instance: *const std::ffi::c_void,
    get_average_amplitude: extern "C" fn(
        *const std::ffi::c_void,
        std::ffi::c_float,
        std::ffi::c_float,
    ) -> std::ffi::c_float,
    get_frequency_amplitude:
        extern "C" fn(*const std::ffi::c_void, std::ffi::c_float) -> std::ffi::c_float,
    get_max_frequency: extern "C" fn(*const std::ffi::c_void) -> std::ffi::c_float,
    free: extern "C" fn(*const std::ffi::c_void),
}

unsafe impl Send for AudioApi {}
unsafe impl Sync for AudioApi {}

impl AudioApi {
    pub fn new(
        instance: *const std::ffi::c_void,

        get_average_amplitude: extern "C" fn(
            *const std::ffi::c_void,
            std::ffi::c_float,
            std::ffi::c_float,
        ) -> std::ffi::c_float,
        get_frequency_amplitude: extern "C" fn(
            *const std::ffi::c_void,
            std::ffi::c_float,
        ) -> std::ffi::c_float,
        get_max_frequency: extern "C" fn(*const std::ffi::c_void) -> std::ffi::c_float,
        free: extern "C" fn(*const std::ffi::c_void),
    ) -> Self {
        Self {
            instance,
            get_average_amplitude,
            get_frequency_amplitude,
            get_max_frequency,
            free,
        }
    }
}

static AUDIO_API_INSTANCE: OnceLock<Mutex<AudioApi>> = OnceLock::new();

pub fn on_load(audio_api: AudioApi) {
    let mut api = AUDIO_API_INSTANCE
        .get_or_init(|| Mutex::new(audio_api))
        .lock()
        .unwrap();
    *api = audio_api;
}

pub fn get_average_amplitude(lower_freq: f32, upper_freq: f32) -> f32 {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();
    (api.get_average_amplitude)(api.instance, lower_freq, upper_freq)
}

pub fn get_frequency_amplitude(frequency: f32) -> f32 {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_frequency_amplitude)(api.instance, frequency)
}

pub fn get_max_frequency() -> std::ffi::c_float {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_max_frequency)(api.instance)
}

pub fn free() {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.free)(api.instance)
}
