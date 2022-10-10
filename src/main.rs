mod audio;
// use audio::start_audio_loop;
mod server_tokio;
fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().build().unwrap();
    let server_handle = server_tokio::main(runtime.handle().clone());
    // let (_stream, rx) = start_audio_loop();

    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        // for patnais in &rx {
        //     println!("{}", patnais);
        // }
    }
    server_handle.abort();
}
