mod audio;
mod config_parser;
mod connections;
mod pipewire_listener;
mod resources;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
};

use anyhow::Result;
use audio::start_audio_loop;
use clap::Parser;
use config_parser::TurboAudioConfig;
use connections::{tcp::TcpConnection, terminal::UsbConnection, Connection};
use pipewire_listener::PipewireController;
use resources::{
    color::Color,
    effects::{
        moody::{update_moody, Moody, MoodySettings},
        raindrop::{update_raindrop, RaindropSettings, RaindropState, Raindrops},
        Effect,
    },
    ledstrip::LedStrip,
    settings::Settings,
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
    let mut effect_settings: HashMap<i32, i32> = HashMap::default();
    let mut connections: HashMap<i32, Connection> = HashMap::default();
    let mut ledstrips = vec![];

    let moody_settings = MoodySettings {
        color: Color { r: 255, g: 0, b: 0 },
    };
    let raindrop_settings = RaindropSettings { rain_speed: 1 };
    settings.insert(0, Settings::Moody(moody_settings));
    settings.insert(1, Settings::Raindrop(raindrop_settings));

    let moody = Moody { id: 10 };
    effects.insert(10, Effect::Moody(moody));
    effect_settings.insert(10, 0);

    let raindrop = Raindrops {
        id: 20,
        state: RaindropState { riples: vec![] },
    };
    effects.insert(20, Effect::Raindrop(raindrop));
    effect_settings.insert(20, 1);

    let ip = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 197), 1234));
    let connection = TcpConnection::new(ip).unwrap();
    let connection_id = 1;
    connections.insert(connection_id, Connection::Tcp(connection));
    connections.insert(2, Connection::Usb(UsbConnection {}));

    let mut ls1 = LedStrip::new();
    ls1.set_led_count(300);
    ls1.add_effect(20, 300);
    ls1.connection_id = Some(connection_id);
    ledstrips.push(ls1);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
        tick(
            &mut ledstrips,
            &mut effects,
            &mut settings,
            &effect_settings,
            &mut connections,
        );
    }
}

fn send_to_connection(ledstrip: &mut LedStrip, connections: &mut HashMap<i32, Connection>) {
    if ledstrip.connection_id.is_none() {
        return;
    }

    let connection_id = ledstrip.connection_id.unwrap();
    let connection = connections
        .get_mut(&connection_id)
        .expect("Failed to find connection");

    let data: Vec<u8> = ledstrip
        .colors
        .iter()
        .flat_map(|color| color.to_bytes())
        .collect();
    match connection {
        Connection::Tcp(tcp_connection) => {
            // If send fails, connection is closed.
            if tcp_connection
                .data_queue
                .as_ref()
                .unwrap()
                .send(data)
                .is_err()
            {
                let connection = connections.remove(&connection_id).unwrap();
                ledstrip.connection_id = None;
                if let Connection::Tcp(connection) = connection {
                    connection.join();
                }
            }
        }
        Connection::Usb(_terminal) => {
            todo!("Implement Usb connection");
        }
    }
}

fn tick(
    ledstrips: &mut Vec<LedStrip>,
    effects: &mut HashMap<i32, Effect>,
    settings: &mut HashMap<i32, Settings>,
    effect_settings: &HashMap<i32, i32>,
    connections: &mut HashMap<i32, Connection>,
) {
    for ledstrip in ledstrips {
        for (effect_id, interval) in &ledstrip.effects {
            let leds = ledstrip
                .colors
                .get_mut(interval.0..=interval.1)
                .expect("Ledstrip interval out of bounds");
            let effect = effects
                .get_mut(effect_id)
                .expect("Effect id was not found.");
            let setting_id = effect_settings
                .get(effect_id)
                .expect("Setting id not found");
            let setting = settings.get_mut(setting_id);
            match (effect, setting) {
                (Effect::Moody(_moody), Some(Settings::Moody(settings))) => {
                    update_moody(leds, settings);
                }
                (Effect::Raindrop(raindrop), Some(Settings::Raindrop(settings))) => {
                    update_raindrop(leds, settings, &mut raindrop.state);
                }
                _ => panic!("Effect doesn't match settings"),
            }
        }
        send_to_connection(ledstrip, connections);
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
    Ok(())
}
