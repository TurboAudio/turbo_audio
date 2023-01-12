mod audio;
mod audio_processing;
mod config_parser;
mod connections;
mod pipewire_listener;
mod resources;
use resources::{
    color::Color,
    effects::{moody::update_moody, raindrop::update_raindrop},
    ledstrip::LedStrip,
};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    thread,
    time::Duration,
};

use anyhow::anyhow;
use audio::start_audio_loop;
use clap::Parser;
use config_parser::TurboAudioConfig;
use connections::{tcp::TcpConnection, usb::UsbConnection, Connection};
use pipewire_listener::PipewireController;

use crate::resources::{
    effects::{
        lua::{LuaEffect, LuaEffectSettings},
        moody::{Moody, MoodySettings},
        raindrop::{RaindropSettings, RaindropState, Raindrops},
        Effect,
    },
    settings::Settings,
};

#[derive(Parser, Debug)]
#[command(author, version, long_about = None)]
struct Args {
    /// Settings file
    #[arg(long, default_value_t = String::from("Settings"))]
    settings_file: String,
}

#[derive(Debug)]
enum RunLoopError {
    LoadEffect,
    LoadConfigFile,
    StartAudioLoop,
    StartPipewireStream,
}

fn test_and_run_loop() -> Result<(), RunLoopError> {
    let mut settings: HashMap<i32, Settings> = HashMap::default();
    let mut effects: HashMap<i32, Effect> = HashMap::default();
    let mut effect_settings: HashMap<i32, i32> = HashMap::default();
    let mut connections: HashMap<i32, Connection> = HashMap::default();
    let mut ledstrips = Vec::default();

    let moody_settings = MoodySettings {
        color: Color { r: 255, g: 0, b: 0 },
    };
    let raindrop_settings = RaindropSettings {
        rain_speed: 1,
        drop_rate: 0.10,
    };
    let lua_settings = LuaEffectSettings {
        settings: serde_json::json!({
            "enable_beep_boops": true,
            "intensity": 11,
        }),
    };
    settings.insert(0, Settings::Moody(moody_settings));
    settings.insert(1, Settings::Raindrop(raindrop_settings));
    settings.insert(2, Settings::Lua(lua_settings));

    let moody = Moody { id: 10 };
    effects.insert(10, Effect::Moody(moody));
    effect_settings.insert(10, 0);

    let raindrop = Raindrops {
        id: 20,
        state: RaindropState { riples: vec![] },
    };
    effects.insert(20, Effect::Raindrop(raindrop));
    effect_settings.insert(20, 1);

    let lua_effect = LuaEffect::new("scripts/fade.lua").map_err(|e| {
        log::error!("{:?}", e);
        RunLoopError::LoadEffect
    })?;

    effects.insert(30, Effect::Lua(lua_effect));
    effect_settings.insert(30, 2);

    let ip = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 200), 1234));
    let connection = TcpConnection::new(ip);
    let connection_id = 1;
    connections.insert(connection_id, Connection::Tcp(connection));
    connections.insert(2, Connection::Usb(UsbConnection {}));

    let mut ls1 = LedStrip::default();
    ls1.set_led_count(300);
    ls1.add_effect(30, 300);
    ls1.connection_id = Some(connection_id);
    ledstrips.push(ls1);

    let mut lag = chrono::Duration::zero();
    let duration_per_tick: chrono::Duration = chrono::Duration::seconds(1) / 60;
    let mut last_loop_start = std::time::Instant::now();
    loop {
        lag = lag
            .checked_add(&chrono::Duration::from_std(last_loop_start.elapsed()).unwrap())
            .unwrap();
        last_loop_start = std::time::Instant::now();
        let current_sleep_duration = std::cmp::max(
            chrono::Duration::zero(),
            duration_per_tick.checked_sub(&lag).unwrap(),
        );
        std::thread::sleep(current_sleep_duration.to_std().unwrap());

        let _update_result =
            update_ledstrips(&mut ledstrips, &mut effects, &effect_settings, &settings);
        let _send_result = send_ledstrip_colors(&mut ledstrips, &mut connections);

        lag = lag.checked_sub(&duration_per_tick).unwrap();
    }
}

fn update_ledstrips(
    ledstrips: &mut [LedStrip],
    effects: &mut HashMap<i32, Effect>,
    effect_settings: &HashMap<i32, i32>,
    settings: &HashMap<i32, Settings>,
) -> anyhow::Result<()> {
    for ledstrip in ledstrips {
        for (effect_id, interval) in &ledstrip.effects {
            let leds = ledstrip
                .colors
                .get_mut(interval.0..=interval.1)
                .ok_or_else(|| anyhow!("Ledstrip interval out of bounds"))?;
            let effect = effects
                .get_mut(effect_id)
                .ok_or_else(|| anyhow!("Effect id was not found."))?;
            let setting_id = effect_settings
                .get(effect_id)
                .ok_or_else(|| anyhow!("Setting id not found"))?;
            let setting = settings.get(setting_id);
            match (effect, setting) {
                (Effect::Moody(_moody), Some(Settings::Moody(settings))) => {
                    update_moody(leds, settings);
                }
                (Effect::Raindrop(raindrop), Some(Settings::Raindrop(settings))) => {
                    update_raindrop(leds, settings, &mut raindrop.state);
                }
                (Effect::Lua(lua), Some(Settings::Lua(settings))) => {
                    if let Err(e) = lua.tick(leds, settings) {
                        log::error!("Error when executing lua function: {:?}", e);
                    }
                }
                _ => panic!("Effect doesn't match settings"),
            }
        }
    }
    Ok(())
}

fn send_ledstrip_colors(
    ledstrips: &mut Vec<LedStrip>,
    connections: &mut HashMap<i32, Connection>,
) -> anyhow::Result<()> {
    for ledstrip in ledstrips {
        if let Some(connection_id) = ledstrip.connection_id {
            let connection = connections
                .get_mut(&connection_id)
                .ok_or_else(|| anyhow!("Connection id \"{}\" doesn't exist", connection_id))?;

            let data = ledstrip
                .colors
                .iter()
                .flat_map(|color| color.to_bytes())
                .collect::<Vec<_>>();
            match connection {
                Connection::Tcp(tcp_connection) => {
                    // If send fails, connection is closed.
                    if let Err(error) = tcp_connection.send_data(data) {
                        log::error!("{:?}", error);
                        connections.remove(&connection_id);
                        ledstrip.connection_id = None;
                    };
                    return Ok(());
                }
                Connection::Usb(_terminal) => {
                    todo!("Implement Usb connection");
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), RunLoopError> {
    pretty_env_logger::init();
    let Args { settings_file } = Args::parse();
    let TurboAudioConfig {
        device_name,
        jack,
        sample_rate,
        stream_connections,
    } = TurboAudioConfig::new(&settings_file).map_err(|e| {
        log::error!("{:?}", e);
        RunLoopError::LoadConfigFile
    })?;

    let (_stream, audio_rx) = start_audio_loop(device_name, jack, sample_rate.try_into().unwrap())
        .map_err(|e| {
            log::error!("{:?}", e);
            RunLoopError::StartAudioLoop
        })?;
    let pipewire_controller = PipewireController::new();
    pipewire_controller
        .set_stream_connections(stream_connections)
        .map_err(|e| {
            log::error!("{:?}", e);
            RunLoopError::StartPipewireStream
        })?;

    let mut audio_processor = audio_processing::AudioSignalProcessor::new();
    audio_processor.start_audio_processing(audio_rx);
    for _ in 0..100 {
        let fft_result = audio_processor.compute_fft();
        if let Some(result) = fft_result {
            let formated = result[..result.len() / 2]
                .iter()
                .cloned()
                .map(|e| e.norm_sqr() as f32)
                .collect::<Vec<_>>();
            let max = formated
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(index, _)| index);
            println!("{:?}", formated);
            println!("{:?}", max);
        }
        thread::sleep(Duration::from_secs(1));
    }

    test_and_run_loop()?;
    Ok(())
}
