mod audio;
mod config_parser;
mod pipewire_listener;
use anyhow::Result;
use audio::start_audio_loop;
use clap::Parser;
use config_parser::TurboAudioConfig;
use pipewire_listener::PipewireController;

#[derive(Parser, Debug)]
#[command(author, version, long_about = None)]
struct Args {
    /// Settings file
    #[arg(long, default_value_t = String::from("Settings"))]
    settings_file: String,
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
    std::thread::sleep(std::time::Duration::from_secs(3));
    pipewire_controller.set_stream_connections(stream_connections)?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        println!("{:#?}", pipewire_controller.get_streams());
        println!("----");
    }

    // Ok(())
}
