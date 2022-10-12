mod audio;
mod config_parser;
mod pipewire_listener;
use anyhow::Result;
use audio::start_audio_loop;
use clap::Parser;
use config_parser::parse_config;
use pipewire_listener::start_pipewire_listener;

#[derive(Parser, Debug)]
#[command(author, version, long_about = None)]
struct Args {
    /// Settings file
    #[arg(long, default_value_t = String::from("Settings"))]
    settings_file: String,
}

fn run_loop() {
    loop {
        // for patnais in &rx {
        //     println!("{}", patnais);
        // }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = parse_config(&args.settings_file)?;
    let (_stream, _rx) = start_audio_loop(
        config.device_name,
        config.jack,
        config.sample_rate.try_into().unwrap(),
    );
    start_pipewire_listener(config.stream_connections);
    run_loop();
    Ok(())
}
