mod audio;
mod config_parser;
mod connections;
mod controller;
mod hot_reloader;
mod plugins;
mod resources;

use crate::hot_reloader::{HotReloader, WatchablePath};
use crate::plugins::general::manager::GeneralPluginManager;
use crate::resources::ledstrip::LedStrip;
use audio::audio_processing::AudioSignalProcessor;
use audio::{audio_stream::start_audio_loop, pipewire_listener::PipewireController};
use clap::Parser;
use config_parser::{ConnectionConfigType, EffectConfigType, SettingsConfigType, TurboAudioConfig};
use connections::{tcp::TcpConnection, usb::UsbConnection, Connection};
use controller::Controller;
use plugins::effects::{
    lua::LuaEffectSettings, native::NativeEffectSettings, Effect, EffectSettings,
};
use std::path::PathBuf;
use std::sync::atomic::{self, AtomicBool};
use std::{fs::File, path::Path};

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

pub static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);

fn run_loop(
    mut audio_processor: AudioSignalProcessor,
    mut controller: Controller,
    mut general_plugin_manager: GeneralPluginManager,
) -> Result<(), RunLoopError> {
    log::info!("Creating watcher on Settings.json");
    let config_hot_reload = HotReloader::new(&[WatchablePath::non_recursive(&PathBuf::from(
        "Settings.json",
    ))]);

    if let Err(e) = &config_hot_reload {
        log::error!("Couldn't start watching the config for hot reload: {e}");
    }

    let config_hot_reload = config_hot_reload.ok();

    let mut lag = chrono::Duration::zero();
    let duration_per_tick: chrono::Duration = chrono::Duration::seconds(1) / 60;
    let mut last_loop_start = std::time::Instant::now();
    loop {
        if SHOULD_QUIT.load(atomic::Ordering::Relaxed) {
            log::info!("Quitting");
            break Ok(());
        }

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
        controller.check_hot_reload();
        controller.update_led_strips();
        controller.send_ledstrip_colors();

        general_plugin_manager.tick_all();

        if let Some(config_hot_reload) = &config_hot_reload {
            if !config_hot_reload.poll_events().is_empty() {
                log::info!("Config changed. Restarting.");
                return Ok(());
            }
        }

        lag = lag.checked_sub(&duration_per_tick).unwrap();
    }
}

fn load_general_plugin_manager(
    config: &TurboAudioConfig,
    audio_processor: &AudioSignalProcessor,
) -> GeneralPluginManager {
    let mut general_plugin_manager = GeneralPluginManager::new(&audio_processor);

    for plugin_path in &config.general_plugins {
        general_plugin_manager.load_native_plugin(Path::new(plugin_path.as_str()));
    }

    general_plugin_manager
}

#[derive(Debug)]
enum LoadControllerError {
    Invalid,
}

fn load_controller(
    config: &TurboAudioConfig,
    audio_processor: &AudioSignalProcessor,
    lua_effects_folder: impl AsRef<Path>,
) -> Result<Controller, LoadControllerError> {
    let mut controller = Controller::new(audio_processor, &lua_effects_folder);
    for connection_config in config.devices.iter() {
        match &connection_config.connection {
            ConnectionConfigType::Tcp(ip) => controller.add_connection(
                connection_config.id,
                Connection::Tcp(TcpConnection::new(*ip)),
            ),
            ConnectionConfigType::Usb() => {
                controller.add_connection(connection_config.id, Connection::Usb(UsbConnection {}))
            }
        }
    }

    for setting_config in config.effect_settings.iter() {
        match &setting_config.setting {
            SettingsConfigType::Lua(settings) => controller.add_settings(
                setting_config.id,
                EffectSettings::Lua(LuaEffectSettings {
                    settings: settings.clone(),
                }),
            ),
            SettingsConfigType::Native => controller.add_settings(
                setting_config.id,
                EffectSettings::Native(NativeEffectSettings {}),
            ),
        }
    }

    for effect_settings in config.effects.iter() {
        match &effect_settings.effect {
            EffectConfigType::Lua(file_name) => {
                let effect_path = lua_effects_folder.as_ref().to_owned().join(file_name);
                controller.add_lua_effect(effect_settings.effect_id, effect_path);
            }
            EffectConfigType::Native(file_name) => {
                let effect_path = std::path::PathBuf::from(file_name);
                controller.add_native_effect(effect_settings.effect_id, effect_path);
            }
        }
        if !controller
            .link_effect_to_settings(effect_settings.effect_id, effect_settings.settings_id)
        {
            return Err(LoadControllerError::Invalid);
        }
    }

    for ledstrip_config in config.ledstrips.iter() {
        let mut ledstrip = LedStrip::default();
        ledstrip.set_led_count(ledstrip_config.size);
        for effect in ledstrip_config.effects.iter() {
            if !ledstrip.add_effect(effect.effect_id, effect.effect_size) {
                return Err(LoadControllerError::Invalid);
            }
        }
        controller.add_led_strip(ledstrip_config.id, ledstrip);
        if !controller
            .link_led_strip_to_connection(ledstrip_config.id, ledstrip_config.connection_id)
        {
            return Err(LoadControllerError::Invalid);
        }
    }

    Ok(controller)
}

fn main() -> Result<(), RunLoopError> {
    env_logger::init();

    ctrlc::set_handler(|| {
        println!();
        log::info!("Received ctrl-c, requesting to quit");
        SHOULD_QUIT.store(true, atomic::Ordering::Relaxed);
    })
    .expect("Couldn't set the CTRL-C handler");

    let Args { settings_file } = Args::parse();

    loop {
        log::info!("Parsing config.");
        let config: TurboAudioConfig =
            serde_json::from_reader(&File::open(settings_file.clone()).unwrap()).unwrap();
        log::info!("Starting audio loop.");
        let (_stream, audio_rx) = start_audio_loop(config.device_name.clone(), config.sample_rate)
            .map_err(|e| {
                log::error!("{:?}", e);
                RunLoopError::StartAudioLoop
            })?;

        log::info!("Creating pipewire listener.");
        let pipewire_controller = PipewireController::new();
        log::info!("Setting pipewire connections.");
        pipewire_controller
            .set_stream_connections(config.stream_connections.clone())
            .map_err(|e| {
                log::error!("{:?}", e);
                RunLoopError::StartPipewireStream
            })?;

        log::info!("Creating audio processor.");
        let fft_buffer_size: usize = 1024;
        let audio_processor =
            AudioSignalProcessor::new(audio_rx, config.sample_rate, fft_buffer_size);

        log::info!("Loading config into controller.");
        let controller = load_controller(&config, &audio_processor, &config.lua_effects_folder)
            .map_err(|e| {
                log::error!("{:?}", e);
                RunLoopError::LoadConfigFile
            })?;

        let general_plugin_manager = load_general_plugin_manager(&config, &audio_processor);

        log::info!("Starting run loop.");
        run_loop(audio_processor, controller, general_plugin_manager)?;
        if SHOULD_QUIT.load(atomic::Ordering::Relaxed) {
            log::info!("Quitting");
            break Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
