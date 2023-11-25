use crate::{
    audio::audio_processing::AudioSignalProcessor,
    effects::{
        lua::{LuaEffect, LuaEffectLoadError},
        Effect,
    },
};

use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error, RecommendedWatcher, RecursiveMode},
    DebouncedEvent, Debouncer,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
};

pub type HotReloadReceiver =
    Receiver<Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>>;

pub fn start_hot_reload_lua_effects(
    lua_effects_folder: impl AsRef<Path>,
) -> Result<(HotReloadReceiver, Debouncer<RecommendedWatcher>), Error> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_millis(50), tx).unwrap();

    debouncer
        .watcher()
        .watch(lua_effects_folder.as_ref(), RecursiveMode::Recursive)?;

    Ok((rx, debouncer))
}

pub fn start_config_hot_reload() -> Result<(HotReloadReceiver, Debouncer<RecommendedWatcher>), Error>
{
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_millis(50), tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new("./Settings.json"), RecursiveMode::NonRecursive)?;

    Ok((rx, debouncer))
}

/// Check if any of the watched lua files were changed, and reload them if they were.
///
/// # Arguments
///
/// * `package_root` - The root of where lua scripts should find other lua files so that they can
/// use `require("path.to.lua.file")`
///
/// * `hot_reload_rx` - The receiver that receives notifications about file changes
///
/// * `effects` - The map of id to Effect
///
/// * `lua_effects_registry` - The registry of loaded lua effects. It's a map of their path on
/// disk pointing to the loaded lua effect in memory.
///
/// * `audio_processor` - The audio processor.
pub fn check_lua_files_changed(
    package_root: impl AsRef<Path>,
    hot_reload_rx: &HotReloadReceiver,
    effects: &mut HashMap<usize, Effect>,
    lua_effects_registry: &HashMap<PathBuf, Vec<usize>>,
    audio_processor: &AudioSignalProcessor,
) {
    if let Ok(Ok(events)) = hot_reload_rx.try_recv() {
        for event in &events {
            let filename: &Path = event.path.as_ref();
            if let Err(e) = on_lua_file_changed(
                filename,
                &package_root,
                effects,
                lua_effects_registry,
                audio_processor,
            ) {
                log::error!("Aborted reloading lua script {e:?}");
            }
        }
    }
}

fn on_lua_file_changed(
    effect_path: impl AsRef<Path>,
    package_root: impl AsRef<Path>,
    effects: &mut HashMap<usize, Effect>,
    lua_effect_registry: &HashMap<PathBuf, Vec<usize>>,
    audio_processor: &AudioSignalProcessor,
) -> Result<(), LuaEffectLoadError> {
    let Ok(path_id) = std::fs::canonicalize(&effect_path) else {
        // Silent error
        return Ok(());
    };

    if let Some(ids) = lua_effect_registry.get(&path_id) {
        for id in ids {
            if let Some(Effect::Lua(_)) = effects.get(id) {
                let lua_effect = LuaEffect::new(&effect_path, &package_root, audio_processor)?;
                effects.insert(*id, Effect::Lua(lua_effect));
                log::info!(
                    "Reloaded effect {} with id {id}",
                    effect_path.as_ref().display()
                );
            }
        }
    }
    Ok(())
}
