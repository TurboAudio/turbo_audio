use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, InputCallbackInfo, SampleFormat, StreamConfig, SupportedStreamConfig};
use retry::{delay::Exponential, retry_with_index};
use std::sync::{self, mpsc::Receiver, mpsc::Sender};

pub fn start_audio_loop(
    device_name: Option<String>,
    use_jack: bool,
    sample_rate: u32,
) -> anyhow::Result<(cpal::Stream, Receiver<i16>)> {
    let audio_device = get_audio_device(device_name, use_jack);
    let input_config = get_input_config(&audio_device, sample_rate);
    let sample_format = input_config.sample_format();
    let config = input_config.into();

    let max_retries: usize = 3;
    retry_with_index(
        Exponential::from_millis(250).take(max_retries),
        |retry_attempt| {
            let stream_result = start_stream(&config, &audio_device, &sample_format);
            match stream_result {
                Ok(result) => {
                    log::trace!("Started audio stream");
                    Ok(result)
                }
                Err(err) => {
                    log::trace!("Failed to start audio stream");
                    if retry_attempt < max_retries.try_into().unwrap() {
                        log::trace!("Retrying to start audio stream...");
                    }
                    Err(err)
                }
            }
        },
    )
    .with_context(|| "Failed to start stream")
}

fn get_audio_device(device_name: Option<String>, use_jack: bool) -> Device {
    let host = if use_jack {
        cpal::host_from_id(
            cpal::available_hosts()
                .into_iter()
                .find(|id| *id == cpal::HostId::Jack)
                .expect("jack host unavailable"),
        )
        .expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    match device_name {
        Some(device_name) => host
            .devices()
            .expect("Host has no audio device")
            .find(|device| device.name().unwrap() == device_name)
            .unwrap_or_else(|| panic!("No suitable audio device found with name {}", &device_name)),
        None => host
            .default_input_device()
            .expect("No default audio input found"),
    }
}

fn get_input_config(audio_device: &Device, sample_rate: u32) -> SupportedStreamConfig {
    audio_device
        .supported_input_configs()
        .expect("Device has no supported input configs")
        .next()
        .expect("Device has no supported input configs")
        .with_sample_rate(cpal::SampleRate(sample_rate))
}

fn build_audio_stream<T: cpal::Sample>(
    audio_device: &Device,
    config: &StreamConfig,
    tx: Sender<i16>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let err_fn = |err| {
        panic!("ERROR: {:?}", err);
    };

    audio_device.build_input_stream(
        config,
        move |data: &[T], _: &InputCallbackInfo| {
            for point in data {
                let _ = tx.send(point.to_i16());
            }
        },
        err_fn,
    )
}

fn start_stream(
    config: &StreamConfig,
    audio_device: &Device,
    sample_format: &SampleFormat,
) -> Result<(cpal::Stream, Receiver<i16>), cpal::BuildStreamError> {
    let (tx, rx) = sync::mpsc::channel();
    let stream = match sample_format {
        SampleFormat::U16 => build_audio_stream::<u16>(audio_device, config, tx),
        SampleFormat::I16 => build_audio_stream::<i16>(audio_device, config, tx),
        SampleFormat::F32 => build_audio_stream::<f32>(audio_device, config, tx),
    }?;

    Ok((stream, rx))
}
