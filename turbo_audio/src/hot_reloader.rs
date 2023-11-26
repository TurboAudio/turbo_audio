use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error, INotifyWatcher, RecursiveMode},
    DebouncedEvent, Debouncer,
};
use std::{path::Path, sync::mpsc::Receiver};

pub struct WatchablePath<'a> {
    path: &'a Path,
    mode: RecursiveMode,
}

impl<'a> WatchablePath<'a> {
    pub fn recursive(path: &'a Path) -> Self {
        Self {
            path,
            mode: RecursiveMode::Recursive,
        }
    }

    #[allow(unused)]
    pub fn non_recursive(path: &'a Path) -> Self {
        Self {
            path,
            mode: RecursiveMode::NonRecursive,
        }
    }
}

pub struct HotReloader {
    _debouncer: Debouncer<INotifyWatcher>,
    rx: Receiver<Result<Vec<DebouncedEvent>, Error>>,
}

impl HotReloader {
    pub fn new(paths: &[WatchablePath]) -> Result<Self, Error> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = new_debouncer(std::time::Duration::from_millis(250), tx).unwrap();

        for path in paths {
            debouncer.watcher().watch(path.path, path.mode)?;
        }

        Ok(Self {
            _debouncer: debouncer,
            rx,
        })
    }

    pub fn poll_events(&self) -> Vec<DebouncedEvent> {
        if let Ok(Ok(events)) = self.rx.try_recv() {
            return events;
        }
        vec![] // No allocation is done here
    }
}
