use crate::{
    audio::audio_processing::{AudioSignalProcessor, FftResult},
    plugins::audio_api::create_audio_api,
};
use libloading::os::unix::{RTLD_LOCAL, RTLD_NOW};
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use thiserror::Error;
use turbo_plugin::{effect_plugin::NativeEffectPluginVTable, Color};

use super::Effect;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error when loading native library: {0}")]
    LoadError(#[from] libloading::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct NativeEffectsManager {
    libraries: HashMap<PathBuf, Arc<Library>>,
    fft_result: Arc<RwLock<FftResult>>,
}

#[derive(Debug)]
struct Library {
    library: Option<libloading::Library>,
    vtable: *const NativeEffectPluginVTable,
}

unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            ((*self.vtable).unload)();
        }
        self.library.take().unwrap().close().unwrap();
        log::info!("Dropping library");
    }
}

impl NativeEffectsManager {
    pub fn new(audio_processor: &AudioSignalProcessor) -> Self {
        Self {
            libraries: Default::default(),
            fft_result: audio_processor.fft_result.clone(),
        }
    }

    pub fn create_effect(&mut self, effect_path: impl AsRef<Path>) -> Result<Effect> {
        let path = std::fs::canonicalize(&effect_path).unwrap();

        let library = match self.libraries.entry(path) {
            std::collections::hash_map::Entry::Occupied(occupied) => occupied.into_mut(),
            std::collections::hash_map::Entry::Vacant(vacant) => {
                let library = Self::load_library(&self.fft_result, vacant.key())?;
                vacant.insert(Arc::new(library))
            }
        };

        let plugin = unsafe { ((*library.vtable).plugin_create)() };
        Ok(Effect::Native(NativeEffect {
            path: effect_path.as_ref().to_owned(),
            pointer: plugin,
            library: Some(library.clone()),
            is_dropped: false,
        }))
    }

    pub fn on_file_changed(&mut self, path: impl AsRef<Path>) {
        self.libraries.remove(&path.as_ref().to_owned());
        log::info!("Reloading library: {}", path.as_ref().display());

        let Ok(library) = Self::load_library(&self.fft_result, path.as_ref()) else {
            log::error!("Error");
            return;
        };

        self.libraries
            .insert(path.as_ref().to_owned(), Arc::new(library));
    }

    pub fn pre_reload_effect(&mut self, effect: &mut NativeEffect) {
        if let Some(library) = &effect.library {
            unsafe {
                ((*library.vtable).plugin_destroy)(effect.pointer);
            }
        }
        effect.is_dropped = true;
        effect.library.take();
    }

    pub fn reload_effect(&mut self, effect: &mut NativeEffect) {
        log::info!("Reloading native effect");
        let Ok(Effect::Native(new_effect)) = self.create_effect(&effect.path) else {
            log::error!("Decaliss");
            return;
        };
        let _ = std::mem::replace(effect, new_effect);
    }

    fn load_library(fft_result: &Arc<RwLock<FftResult>>, path: &Path) -> Result<Library> {
        unsafe {
            let library = libloading::os::unix::Library::open(Some(path), RTLD_NOW | RTLD_LOCAL)?;

            let vtable_fn =
                library.get::<extern "C" fn() -> *const std::ffi::c_void>(b"_plugin_vtable")?;

            let vtable =
                vtable_fn() as *const turbo_plugin::effect_plugin::NativeEffectPluginVTable;

            let audio_api = create_audio_api(fft_result.clone());

            ((*vtable).load)(audio_api);

            Ok(Library {
                library: Some(library.into()),
                vtable,
            })
        }
    }
}

#[derive(Debug)]
pub struct NativeEffectSettings {}

#[derive(Debug)]
pub struct NativeEffect {
    path: PathBuf,
    pointer: *mut std::ffi::c_void,
    library: Option<Arc<Library>>,
    is_dropped: bool,
}

impl Drop for NativeEffect {
    fn drop(&mut self) {
        if self.is_dropped {
            return;
        }

        if let Some(library) = &self.library {
            log::info!("Dropping native effect");
            unsafe {
                ((*library.vtable).plugin_destroy)(self.pointer);
            }
        } else {
            log::error!("Couldn't drop effect because the library isn't loaded");
        }
    }
}

impl NativeEffect {
    pub fn tick(&mut self, leds: &mut [Color]) -> Result<()> {
        if let Some(library) = &self.library {
            unsafe {
                ((*library.vtable).tick)(self.pointer, leds.as_mut_ptr(), leds.len() as _);
            }
        }
        Ok(())
    }
}
