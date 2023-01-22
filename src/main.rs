mod audio;
mod audio_processing;
mod config_parser;
mod connections;
mod hot_reload;
mod pipewire_listener;
mod resources;
mod controller;

use audio_processing::AudioSignalProcessor;
use controller::Controller;
use hot_reload::{check_lua_files_changed, start_hot_reload_lua_effects};
use resources::{
    color::Color,
    ledstrip::LedStrip,
};
use std::net::{Ipv4Addr, SocketAddrV4};

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

fn test_and_run_loop(mut audio_processor: AudioSignalProcessor) -> Result<(), RunLoopError> {
    // let mut settings: HashMap<usize, Settings> = HashMap::new();
    // let mut effects: HashMap<usize, Effect> = HashMap::new();
    // let mut effect_settings: HashMap<usize, usize> = HashMap::new();
    // let mut connections: HashMap<usize, Connection> = HashMap::new();
    // let mut ledstrips = Vec::new();
    // let mut lua_effects_registry: HashMap<String, usize> = HashMap::default();
    let mut controller = Controller::new();


    // let moody_id = register_effect(
    //     Effect::Moody(moody),
    //     &mut effects,
    //     &mut lua_effects_registry,
    // );
    // effect_settings.insert(moody_id, 0);

    // let raindrops_id = register_effect(
    //     Effect::Raindrop(raindrop),
    //     &mut effects,
    //     &mut lua_effects_registry,
    // );
    // effect_settings.insert(raindrops_id, 1);


    // let lua_id = register_effect(
    //     Effect::Lua(lua_effect),
    //     &mut effects,
    //     &mut lua_effects_registry,
    // );
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

    let ip = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 10), 1234));
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

    // controller.link_led_strip_to_connection(led_strip_id, connection_id)
    // ls1.connection_id = Some(connection_id);
    // ledstrips.push(ls1);

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
        let _ = controller.send_ledstrip_colors();
        // let _update_result =
        //     update_ledstrips(&mut ledstrips, &mut effects, &effect_settings, &settings);
        // let _send_result = send_ledstrip_colors(&mut ledstrips, &mut connections);

        check_lua_files_changed(
            &hot_reload_rx,
            &mut controller.effects,
            &controller.lua_effects_registry,
            &audio_processor,
        );

        lag = lag.checked_sub(&duration_per_tick).unwrap();
    }
}

// fn update_ledstrips(
//     ledstrips: &mut [LedStrip],
//     effects: &mut HashMap<usize, Effect>,
//     effect_settings: &HashMap<usize, usize>,
//     settings: &HashMap<usize, Settings>,
// ) -> anyhow::Result<()> {
//     for ledstrip in ledstrips {
//         for (effect_id, interval) in &ledstrip.effects {
//             let leds = ledstrip
//                 .colors
//                 .get_mut(interval.0..=interval.1)
//                 .ok_or_else(|| anyhow!("Ledstrip interval out of bounds"))?;
//             let effect = effects
//                 .get_mut(effect_id)
//                 .ok_or_else(|| anyhow!("Effect id was not found."))?;
//             let setting_id = effect_settings
//                 .get(effect_id)
//                 .ok_or_else(|| anyhow!("Setting id not found"))?;
//             let setting = settings.get(setting_id);
//             match (effect, setting) {
//                 (Effect::Moody(_moody), Some(Settings::Moody(settings))) => {
//                     update_moody(leds, settings);
//                 }
//                 (Effect::Raindrop(raindrop), Some(Settings::Raindrop(settings))) => {
//                     update_raindrop(leds, settings, &mut raindrop.state);
//                 }
//                 (Effect::Lua(lua), Some(Settings::Lua(settings))) => {
//                     if let Err(e) = lua.tick(leds, settings) {
//                         log::error!("Error when executing lua function: {:?}", e);
//                     }
//                 }
//                 _ => panic!("Effect doesn't match settings"),
//             }
//         }
//     }
//     Ok(())
// }
//
// fn send_ledstrip_colors(
//     ledstrips: &mut Vec<LedStrip>,
//     connections: &mut HashMap<usize, Connection>,
//     ledstrip_connections: &mut HashMap<usize, usize>,
//     ledstrip_id: usize,
// ) -> anyhow::Result<()> {
//     for ledstrip in ledstrips {
//         if let Some(connection_id) = ledstrip_connections.get(&ledstrip_id) {
//             if let Some(connection) = connections.get(connection_id) {
//                 let data = ledstrip
//                     .colors
//                     .iter()
//                     .flat_map(|color| color.to_bytes())
//                     .collect::<Vec<_>>();
//                 match connection {
//                     Connection::Tcp(tcp_connection) => {
//                         // If send fails, connection is closed.
//                         if let Err(error) = tcp_connection.send_data(data) {
//                             log::error!("{:?}", error);
//                             connections.remove(&connection_id);
//                             ledstrip_connections.remove(&ledstrip_id);
//                         }
//                         return Ok(());
//                     }
//                     Connection::Usb(_terminal) => {
//                         todo!("Implement Usb connection");
//                     }
//                 }
//             }
//         }
//     }
//     Ok(())
// }

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

    let (_stream, audio_rx) = start_audio_loop(device_name, jack, sample_rate).map_err(|e| {
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

    let fft_buffer_size: usize = 1024;
    let audio_processor =
        audio_processing::AudioSignalProcessor::new(audio_rx, sample_rate, fft_buffer_size);

    test_and_run_loop(audio_processor)?;
    Ok(())
}
