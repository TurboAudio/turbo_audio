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
    get_bin_count: extern "C" fn(*const std::ffi::c_void) -> std::ffi::c_ulonglong,
    get_bin_value_at_index:
        extern "C" fn(*const std::ffi::c_void, std::ffi::c_ulonglong) -> std::ffi::c_float,
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

        get_bin_count: extern "C" fn(*const std::ffi::c_void) -> std::ffi::c_ulonglong,
        get_bin_value_at_index: extern "C" fn(
            *const std::ffi::c_void,
            std::ffi::c_ulonglong,
        ) -> std::ffi::c_float,
        free: extern "C" fn(*const std::ffi::c_void),
    ) -> Self {
        Self {
            instance,
            get_average_amplitude,
            get_frequency_amplitude,
            get_max_frequency,
            get_bin_count,
            get_bin_value_at_index,
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

pub fn get_max_frequency() -> f32 {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_max_frequency)(api.instance)
}

pub fn get_bin_count() -> usize {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_bin_count)(api.instance) as _
}

pub fn get_bin_value_at_index(index: usize) -> f32 {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    (api.get_bin_value_at_index)(api.instance, index as _)
}

pub fn get_all_bins(bins: &mut Vec<f32>) {
    let Some(api) = AUDIO_API_INSTANCE.get() else {
        eprintln!("PLUGIN ERROR: Couldn't get the audio api pointer");
        abort();
    };
    let api = api.lock().unwrap();

    let bin_count: usize = (api.get_bin_count)(api.instance) as usize / 2;
    bins.resize(bin_count, 0.0_f32);

    for (index, value) in bins.iter_mut().enumerate() {
        *value = (api.get_bin_value_at_index)(api.instance, index as _);
    }
}

pub fn free() {
    if let Some(api) = AUDIO_API_INSTANCE.get() {
        let api = api.lock().unwrap();
        (api.free)(api.instance);
    }
}
