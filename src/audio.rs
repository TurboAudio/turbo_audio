use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, InputCallbackInfo, SampleFormat, StreamConfig, SupportedStreamConfig};
use std::sync::{self, mpsc::Receiver, mpsc::Sender};

pub fn start_audio_loop() -> (cpal::Stream, Receiver<i16>) {
    let audio_device = get_audio_device();
    let input_config = get_input_config(&audio_device);
    let sample_format = input_config.sample_format();
    let config = input_config.into();

    println!("Sample format: {:?}", sample_format);

    start_stream(&config, &audio_device, &sample_format)
}

fn get_audio_device() -> Device {
    let host = cpal::default_host();
    let mut devices = host.devices().expect("Host has no audio device");
    devices
        .find(|device| device.name().expect("Failed to retrieve audio device name") == "pulse")
        .expect("No suitable audio device found")
}

fn get_input_config(audio_device: &Device) -> SupportedStreamConfig {
    audio_device
        .supported_input_configs()
        .expect("Device has no supported input configs")
        .next()
        .expect("Device has no supported input configs")
        .with_sample_rate(cpal::SampleRate(44100))
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
) -> (cpal::Stream, Receiver<i16>) {
    let (tx, rx) = sync::mpsc::channel();

    let stream = match sample_format {
        SampleFormat::U16 => build_audio_stream::<u16>(audio_device, config, tx),
        SampleFormat::I16 => build_audio_stream::<i16>(audio_device, config, tx),
        SampleFormat::F32 => build_audio_stream::<f32>(audio_device, config, tx),
    }
    .unwrap();

    (stream, rx)
}
