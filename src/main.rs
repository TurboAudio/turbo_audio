mod audio;
mod config_parser;
mod pipewire_listener;
mod resources;
use resources::{
    color::Color,
    effect::{update_moody, update_raindrop, Effect, Moody},
    ledstrip::LedStrip,
    settings::{MoodySettings, Settings},
};
use std::collections::HashMap;

use anyhow::Result;
use audio::start_audio_loop;
use clap::Parser;
use config_parser::TurboAudioConfig;
use pipewire_listener::PipewireController;

use crate::resources::{
    effect::{RaindropState, Raindrops},
    settings::RaindropSettings,
};

#[derive(Parser, Debug)]
#[command(author, version, long_about = None)]
struct Args {
    /// Settings file
    #[arg(long, default_value_t = String::from("Settings"))]
    settings_file: String,
}

fn test_and_run_loop() {
    let mut settings: HashMap<i32, Settings> = HashMap::default();
    let mut effects: HashMap<i32, Effect> = HashMap::default();
    let mut ledstrips = vec![];

    let moody_settings = MoodySettings {
        color: Color { r: 255, g: 0, b: 0 },
    };
    let raindrop_settings = RaindropSettings { rain_speed: 1 };
    settings.insert(0, Settings::Moody(moody_settings));
    settings.insert(1, Settings::Raindrop(raindrop_settings));

    let moody = Moody {
        id: 10,
        settings_id: 0,
    };
    let raindrop = Raindrops {
        id: 20,
        settings_id: 1,
        state: RaindropState { riples: vec![] },
    };
    effects.insert(10, Effect::Moody(moody));
    effects.insert(20, Effect::Raindrop(raindrop));

    let mut ls1 = LedStrip::new();
    ls1.set_led_count(10);
    ls1.add_effect(20, 10);
    ledstrips.push(ls1);

    for _ in 0..10 {
        println!("{:?}", ledstrips.get(0).unwrap().colors);
        tick(&mut ledstrips, &mut effects, &mut settings);
    }
    println!("{:?}", ledstrips);
    
    tick(&mut ledstrips, &mut effects, &mut settings);
    println!("{:?}", ledstrips);
    
    settings.get_mut(&0).unwrap().mut_moody().color = Color {r: 255, g:255, b:255};
    tick(&mut ledstrips, &mut effects, &mut settings);
    println!("{:?}", ledstrips);
    
    ledstrips.get_mut(0).unwrap().set_led_count(3);
    tick(&mut ledstrips, &mut effects, &mut settings);
    println!("{:?}", ledstrips);
    
    ledstrips.get_mut(0).unwrap().set_led_count(10);
    tick(&mut ledstrips, &mut effects, &mut settings);
    println!("{:?}", ledstrips);
}

fn tick(
    ledstrips: &mut Vec<LedStrip>,
    effects: &mut HashMap<i32, Effect>,
    settings: &mut HashMap<i32, Settings>,
) {
    for strip in ledstrips {
        for (effect_id, interval) in &strip.effects {
            let leds = strip
                .colors
                .get_mut(interval.0..=interval.1)
                .expect("Ledstrip interval out of bounds");
            match effects
                .get_mut(effect_id)
                .expect("Effect id was not found.")
            {
                Effect::Moody(moody) => {
                    let settings = settings
                        .get_mut(&moody.settings_id)
                        .expect("Setting id was not found.");
                    update_moody(leds, settings.moody());
                }
                Effect::Raindrop(raindrop) => {
                    let settings = settings
                        .get_mut(&raindrop.settings_id)
                        .expect("Setting id was not found.");
                    update_raindrop(leds, settings.raindrop(), &mut raindrop.state);
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let Args { settings_file } = Args::parse();
    let TurboAudioConfig {
        device_name,
        jack,
        sample_rate,
        stream_connections,
    } = TurboAudioConfig::new(&settings_file)?;

    let (_stream, _rx) = start_audio_loop(device_name, jack, sample_rate.try_into().unwrap())?;
    let pipewire_controller = PipewireController::new();
    pipewire_controller.set_stream_connections(stream_connections)?;
    test_and_run_loop();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    Ok(())
}
