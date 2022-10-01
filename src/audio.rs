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
        .find(|device| device.name().expect("Audio device has no name") == "pulse")
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

fn callback<T: cpal::Sample>(data: &[T], tx: &Sender<i16>) {
    for point in data {
        let _ = tx.send(point.to_i16());
    }
}

fn start_stream(
    config: &StreamConfig,
    audio_device: &Device,
    sample_format: &SampleFormat,
) -> (cpal::Stream, Receiver<i16>) {
    let err_fn = |err| {
        panic!("ERROR: {:?}", err);
    };

    let (tx, rx) = sync::mpsc::channel();

    let stream = match sample_format {
        SampleFormat::U16 => audio_device.build_input_stream(
            config,
            move |data: &[u16], _: &InputCallbackInfo| {
                callback(data, &tx);
            },
            err_fn,
        ),
        SampleFormat::I16 => audio_device.build_input_stream(
            config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                callback(data, &tx);
            },
            err_fn,
        ),
        SampleFormat::F32 => audio_device.build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                callback(data, &tx);
            },
            err_fn,
        ),
    }
    .unwrap();

    (stream, rx)
}
