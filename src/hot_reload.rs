use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error, RecommendedWatcher, RecursiveMode},
    DebouncedEvent, Debouncer,
};
use std::{path::Path, sync::mpsc::Receiver};

pub type HotReloadReceiver = Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>;

pub fn start_hot_reload_lua_effects(
) -> Result<(HotReloadReceiver, Debouncer<RecommendedWatcher>), Error> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_millis(50), None, tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new("./scripts/"), RecursiveMode::Recursive)?;

    Ok((rx, debouncer))
}

