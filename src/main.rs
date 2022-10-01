use cpal::traits::{DeviceTrait, HostTrait};
// use cpal::Data;

fn get_audio_device() -> Option<cpal::Device> {
    let host = cpal::default_host();
    let mut devices = host.devices().expect("Host has no audio device");
    devices.find(|device| device.name().expect("Audio device has no name") == "pulse")
}

fn get_input_config(audio_device: &cpal::Device) -> cpal::SupportedStreamConfig {
    audio_device
        .supported_input_configs()
        .expect("Device has no supported input configs")
        .next()
        .expect("Device has no supported input configs")
        .with_sample_rate(cpal::SampleRate(44100))
}

fn read_audio_stream<T: cpal::Sample + std::fmt::Display>(data: &[T], _: &cpal::InputCallbackInfo) {
    for point in data {
        println!("{}", point.to_f32());
    }
}

fn main() {
    let audio_device = get_audio_device().expect("No suitable audio device found");
    let input_config = get_input_config(&audio_device);
    let sample_format = input_config.sample_format();
    let config = input_config.into();

    println!("Sample format: {:?}", sample_format);

    let err_fn = |err| {
        panic!("ERROR: {:?}", err);
    };

    let _stream = match sample_format {
        cpal::SampleFormat::U16 => {
            audio_device.build_input_stream(&config, read_audio_stream::<u16>, err_fn)
        }
        cpal::SampleFormat::I16 => {
            audio_device.build_input_stream(&config, read_audio_stream::<i16>, err_fn)
        }
        cpal::SampleFormat::F32 => {
            audio_device.build_input_stream(&config, read_audio_stream::<f32>, err_fn)
        }
    }
    .unwrap();

    println!("This is a test!");

    loop {
        std::thread::sleep(std::time::Duration::new(5, 0));
    }
}
