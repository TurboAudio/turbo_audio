mod server_tokio;
use server_tokio::WebSocketServer;

mod audio;
use audio::start_audio_loop;
fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().build().unwrap();
    let mut server = WebSocketServer::new();
    server.start_server(runtime.handle().clone());
    let (_stream, rx) = start_audio_loop();

    for _ in 0..60 {
        for patnais in &rx {
            println!("{}", patnais);
        }
    }
    server.close();
}
