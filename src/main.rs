mod audio;
use audio::start_audio_loop;

fn main() {
    let (_stream, rx) = start_audio_loop();

    loop {
        for patnais in &rx {
            println!("{}", patnais);
        }
    }
}
