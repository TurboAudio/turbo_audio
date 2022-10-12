mod server;
use std::{thread, time};

use server::WebSocketServer;

mod audio;
use audio::start_audio_loop;
fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().build().unwrap();
    let mut server = WebSocketServer::new();
    server.start_server(runtime.handle().clone());
    let (_stream, rx) = start_audio_loop();

    loop {
        thread::sleep(time::Duration::from_secs(1));
        // for patnais in &rx {
        //     println!("{}", patnais);
        // }
    }
    server.close();
}
