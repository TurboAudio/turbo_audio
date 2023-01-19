// use ;
use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error, RecursiveMode, RecommendedWatcher},
    DebouncedEvent,
    Debouncer,
};
use std::path::Path;
use std::sync::mpsc::Receiver;

pub fn start_hot_reload() -> Result<(Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>, Debouncer<RecommendedWatcher>), Error> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_secs(1), None, tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new("."), RecursiveMode::Recursive)?;

    Ok((rx, debouncer))
}

// /// Example for debouncer
// fn main() {
// }
