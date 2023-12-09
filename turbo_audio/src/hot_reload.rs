use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error, RecommendedWatcher, RecursiveMode},
    DebouncedEvent, Debouncer,
};
use std::{path::Path, sync::mpsc::Receiver};

pub type HotReloadReceiver =
    Receiver<Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>>;

pub fn start_config_hot_reload() -> Result<(HotReloadReceiver, Debouncer<RecommendedWatcher>), Error>
{
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_millis(50), tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new("./Settings.json"), RecursiveMode::NonRecursive)?;

    Ok((rx, debouncer))
}
