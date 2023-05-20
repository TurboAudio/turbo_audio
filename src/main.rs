mod audio;
mod audio_config;
mod audio_processing;
mod config_parser;
mod connections;
mod controller;
mod hot_reload;
mod pipewire_listener;
mod resources;

use audio_config::AudioConfig;
use audio_processing::AudioSignalProcessor;
use config_parser::parse_config;
use controller::Controller;
use hot_reload::{check_lua_files_changed, start_hot_reload_lua_effects};
use resources::{color::Color, ledstrip::LedStrip};
use std::{
    fs::File,
    net::{Ipv4Addr, SocketAddrV4},
};

use audio::start_audio_loop;
use clap::Parser;
use config_parser::TurboAudioConfig;
use connections::{tcp::TcpConnection, usb::UsbConnection, Connection};
use pipewire_listener::{PipewireController, PortConnections, StreamConnections};

use crate::resources::{effects::Effect, settings::Settings};

#[derive(Parser, Debug)]
#[command(author, version, long_about = None)]
struct Args {
    /// Settings file
    #[arg(long, default_value_t = String::from("Settings.json"))]
    settings_file: String,
}

#[derive(Debug)]
enum RunLoopError {
    LoadConfigFile,
    StartAudioLoop,
    StartPipewireStream,
}

fn run_loop(
    mut audio_processor: AudioSignalProcessor,
    mut controller: Controller,
) -> Result<(), RunLoopError> {
    let mut controller = Controller::new();

    let lua_id: usize = 1;
    let lua_effect = LuaEffect::new("scripts/sketchers.lua", &audio_processor).map_err(|e| {
        log::error!("{:?}", e);
        RunLoopError::LoadEffect
    })?;
    let lua_setting_id: usize = 1;
    let lua_settings = LuaEffectSettings {
        settings: serde_json::json!({
            "enable_beep_boops": true,
            "intensity": 11,
        }),
    };
    controller.add_effect(lua_id, Effect::Lua(lua_effect));
    controller.add_settings(lua_setting_id, Settings::Lua(lua_settings));
    controller.link_effect_to_settings(lua_id, lua_setting_id);

    let moody_id: usize = 2;
    let moody = Moody {};
    let moody_settings_id: usize = 2;
    let moody_settings = MoodySettings {
        color: Color { r: 255, g: 0, b: 0 },
    };
    controller.add_settings(moody_settings_id, Settings::Moody(moody_settings));
    controller.add_effect(moody_id, Effect::Moody(moody));
    controller.link_effect_to_settings(moody_id, moody_settings_id);

    let raindrop_id: usize = 3;
    let raindrop = Raindrops {
        state: RaindropState { riples: vec![] },
    };
    let raindrop_settings_id = 3;
    let raindrop_settings = RaindropSettings {
        rain_speed: 1,
        drop_rate: 0.10,
    };
    controller.add_settings(raindrop_settings_id, Settings::Raindrop(raindrop_settings));
    controller.add_effect(raindrop_id, Effect::Raindrop(raindrop));
    controller.link_effect_to_settings(raindrop_id, raindrop_settings_id);

    let ip = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 159), 1234));
    // let ip = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 42069));
    let connection = TcpConnection::new(ip);
    let connection_id = 1;
    controller.add_connection(connection_id, Connection::Tcp(connection));
    controller.add_connection(2, Connection::Usb(UsbConnection {}));

    let mut ls1 = LedStrip::default();
    let led_strip_id: usize = 1;
    ls1.set_led_count(300);
    ls1.add_effect(lua_id, 300);
    controller.add_led_strip(led_strip_id, ls1);
    controller.link_led_strip_to_connection(led_strip_id, connection_id);

    if let Err(e) = start_hot_reload_lua_effects() {
        log::error!("Hot reload may not be active: {e:?}");
    }
    let (hot_reload_rx, _debouncer) = start_hot_reload_lua_effects().unwrap();

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
        audio_processor.compute_fft();

        let _fft_result_read_lock = audio_processor.fft_result.read().unwrap();
        controller.update_led_strips();
        controller.send_ledstrip_colors();

        check_lua_files_changed(
            &hot_reload_rx,
            &mut controller.effects,
            &controller.lua_effects_registry,
            &audio_processor,
        );

        lag = lag.checked_sub(&duration_per_tick).unwrap();
    }
}

fn map_and_log_error<T, ErrorType: std::fmt::Debug, NewErrorType>(
    result: Result<T, ErrorType>,
    mapped_error: NewErrorType,
) -> Result<T, NewErrorType> {
    result.map_err(|e| {
        log::error!("{:?}", e);
        mapped_error
    })
}

fn main() -> Result<(), RunLoopError> {
    env_logger::init();
    let Args { settings_file } = Args::parse();
    let config: TurboAudioConfig = serde_json::from_reader(&File::open(settings_file).unwrap()).unwrap();
    let TurboAudioConfig {
        device_name,
        jack,
        sample_rate,
        stream_connections,
    } = config;

    let (_stream, audio_rx) = map_and_log_error(
        start_audio_loop(device_name, jack, sample_rate),
        RunLoopError::StartAudioLoop,
    )?;

    let pipewire_controller = PipewireController::new();
    log::info!("Eille");
    pipewire_controller
        .set_stream_connections(stream_connections)
        .map_err(|e| {
            log::error!("{:?}", e);
            RunLoopError::StartPipewireStream
        })?;

    let fft_buffer_size: usize = 1024;
    let audio_processor =
        AudioSignalProcessor::new(audio_rx, sample_rate, fft_buffer_size);

    let controller = map_and_log_error(
        parse_config(&settings_file, &audio_processor),
        RunLoopError::LoadConfigFile,
    )?;

    run_loop(audio_processor, controller)?;
    Ok(())
}
