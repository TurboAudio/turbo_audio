use turbo_plugin::audio_api::AudioApi;

use crate::audio::audio_processing::FftResult;
use std::{
    boxed::Box,
    sync::{Arc, RwLock},
};

pub fn create_audio_api(fft_result: Arc<RwLock<FftResult>>) -> AudioApi {
    extern "C" fn get_average_amplitude(
        instance: *const std::ffi::c_void,
        lower_frequency: std::ffi::c_float,
        upper_frequency: std::ffi::c_float,
    ) -> std::ffi::c_float {
        let fft_result = unsafe { &*(instance as *const Arc<RwLock<FftResult>>) };
        fft_result
            .read()
            .unwrap()
            .get_average_amplitude(lower_frequency, upper_frequency)
            .unwrap_or_else(|| {
                log::error!("Invalid frequencies: {lower_frequency} & {upper_frequency}");
                0.0f32
            })
    }

    extern "C" fn get_frequency_amplitude(
        instance: *const std::ffi::c_void,
        frequency: std::ffi::c_float,
    ) -> std::ffi::c_float {
        let fft_result = unsafe { &*(instance as *const Arc<RwLock<FftResult>>) };
        fft_result
            .read()
            .unwrap()
            .get_frequency_amplitude(frequency)
            .unwrap_or_else(|| {
                log::error!("Invalid frequency: {frequency}");
                0.0f32
            })
    }

    extern "C" fn get_max_frequency(instance: *const std::ffi::c_void) -> std::ffi::c_float {
        let fft_result = unsafe { &*(instance as *const Arc<RwLock<FftResult>>) };
        fft_result.read().unwrap().get_max_frequency()
    }

    extern "C" fn free(instance: *const std::ffi::c_void) {
        unsafe {
            drop(Box::from_raw(instance as *mut Arc<RwLock<FftResult>>));
        }
    }

    let fft_result = Box::new(fft_result);

    AudioApi::new(
        Box::into_raw(fft_result) as *const _,
        get_average_amplitude,
        get_frequency_amplitude,
        get_max_frequency,
        free,
    )
}
