mod audio;
use audio::start_audio_loop;
use clap::Parser;

/// Haha brr
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the input audio device to choose
    #[arg(long)]
    device_name: Option<String>,

    /// Toggle if Jack should be used as the audio host
    #[arg(long, default_value_t = false, action = clap::ArgAction::SetTrue)]
    jack: bool,

    /// Sample rate of the input stream
    #[arg(long, default_value_t = 48000)]
    sample_rate: u32,
}

fn main() {
    let args = Args::parse();
    let (_stream, rx) = start_audio_loop(args.device_name, args.jack, args.sample_rate);

    loop {
        for patnais in &rx {
            println!("{}", patnais);
        }
    }
}
