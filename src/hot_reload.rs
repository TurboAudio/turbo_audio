use crate::{
    audio_processing::AudioSignalProcessor,
    resources::effects::{
        lua::{LuaEffect, LuaEffectLoadError},
        Effect,
    },
};

use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error, RecommendedWatcher, RecursiveMode},
    DebouncedEvent, Debouncer,
};
use std::{collections::HashMap, path::Path, sync::mpsc::Receiver};

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

pub fn check_lua_files_changed(
    hot_reload_rx: &HotReloadReceiver,
    effects: &mut HashMap<usize, Effect>,
    lua_effects_registry: &HashMap<String, Vec<usize>>,
    audio_processor: &AudioSignalProcessor,
) {
    if let Ok(Ok(events)) = hot_reload_rx.try_recv() {
        for event in &events {
            if let Some(filename) = event.path.to_str() {
                if let Err(e) =
                    on_lua_file_changed(filename, effects, lua_effects_registry, audio_processor)
                {
                    log::error!("Aborted reloading lua script {e:?}");
                }
            }
        }
    }
}

fn on_lua_file_changed(
    filename: &str,
    effects: &mut HashMap<usize, Effect>,
    lua_effect_registry: &HashMap<String, Vec<usize>>,
    audio_processor: &AudioSignalProcessor,
) -> Result<(), LuaEffectLoadError> {
    if let Some(start) = filename.find("/./") {
        let filename = &filename[start + 3..];
        if let Some(ids) = lua_effect_registry.get(filename) {
            for id in ids {
                if let Some(Effect::Lua(_)) = effects.get(id) {
                    let lua_effect = LuaEffect::new(filename, audio_processor)?;
                    effects.insert(*id, Effect::Lua(lua_effect));
                    log::info!("Reloaded effect {filename} with id {id}");
                }
            }
        }
    }
    Ok(())
}
