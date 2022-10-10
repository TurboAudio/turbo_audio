mod audio;
use audio::start_audio_loop;
mod server_tokio;
#[tokio::main]
async fn main() {
    server_tokio::main().await;
    let (_stream, rx) = start_audio_loop();

    loop {
        for patnais in &rx {
            println!("{}", patnais);
        }
    }
}
