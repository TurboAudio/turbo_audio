use cpal::traits::{DeviceTrait, HostTrait};

mod effect;
use crate::effect::{Effect, MoodyEffect};

fn main() {
    let host = cpal::default_host();
    let devices = host.devices().unwrap();

    for device in devices {
        let supported_input_configs = device.supported_input_configs();
        if supported_input_configs.is_err() {
            continue;
        }

        let supported_input_configs = supported_input_configs.unwrap().next();
        if supported_input_configs.is_none() {
            continue;
        }

        let config = supported_input_configs.unwrap().with_max_sample_rate();

        if device.name().unwrap() == "pulse" {
            println!("Device: {}, config: {:?}", device.name().unwrap(), config);
            break;
        }
    }
    effect_test();
}

fn effect_test() {
    let moody_effect = effect::MoodyEffect::new();
    let effect_string = moody_effect.serialize();
    let new_effect: MoodyEffect = effect::MoodyEffect::deserialize(&effect_string);
    println!("{}", effect_string);
}
