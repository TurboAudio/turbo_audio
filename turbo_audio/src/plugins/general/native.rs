use libloading::os::unix::{RTLD_LOCAL, RTLD_NOW};
use std::{
    collections::HashMap,
    ffi::CStr,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use thiserror::Error;
use turbo_plugin::general_plugin::NativeGeneralPluginVTable;

use crate::{
    audio::audio_processing::{AudioSignalProcessor, FftResult},
    plugins::audio_api::create_audio_api,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error when loading native library: {0}")]
    LoadError(#[from] libloading::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct NativeGeneralPluginManager {
    libraries: HashMap<PathBuf, Arc<Library>>,
    fft_result: Arc<RwLock<FftResult>>,
}

#[derive(Debug)]
struct Library {
    library: Option<libloading::Library>,
    vtable: *const NativeGeneralPluginVTable,
    filename: PathBuf,
}

unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            ((*self.vtable).unload)();
        }
        self.library.take().unwrap().close().unwrap();
        log::info!("Dropping library: {}", self.filename.display());
    }
}

impl NativeGeneralPluginManager {
    pub fn new(audio_processor: &AudioSignalProcessor) -> Self {
        Self {
            libraries: Default::default(),
            fft_result: audio_processor.fft_result.clone(),
        }
    }

    pub fn create_plugin(&mut self, plugin_path: impl AsRef<Path>) -> Result<NativeGeneralPlugin> {
        let path = std::fs::canonicalize(&plugin_path).unwrap();

        // Load the shared object, or simply retrieve it from memory if already loaded
        let library = match self.libraries.entry(path) {
            std::collections::hash_map::Entry::Occupied(occupied) => occupied.into_mut(),
            std::collections::hash_map::Entry::Vacant(vacant) => {
                let library = Self::load_shared_library(&self.fft_result, vacant.key())?;
                log::info!("Loaded shared object: {}", plugin_path.as_ref().display());
                vacant.insert(Arc::new(library))
            }
        };

        // Instanciate the plugin itself
        let plugin = unsafe { ((*library.vtable).plugin_create)() };
        Ok(NativeGeneralPlugin {
            path: plugin_path.as_ref().to_owned(),
            plugin_pointer: plugin,
            library: Some(library.clone()),
            is_dropped: false,
        })
    }

    fn load_shared_library(fft_result: &Arc<RwLock<FftResult>>, path: &Path) -> Result<Library> {
        unsafe {
            let library = libloading::os::unix::Library::open(Some(path), RTLD_NOW | RTLD_LOCAL)?;

            let vtable_fn =
                library.get::<extern "C" fn() -> *const std::ffi::c_void>(b"_plugin_vtable")?;

            let vtable = vtable_fn() as *const NativeGeneralPluginVTable;

            let audio_api = create_audio_api(fft_result.clone());

            ((*vtable).load)(audio_api);

            Ok(Library {
                library: Some(library.into()),
                vtable,
                filename: path.to_owned(),
            })
        }
    }
}

#[derive(Debug)]
pub struct NativeGeneralPlugin {
    path: PathBuf,
    plugin_pointer: *mut std::ffi::c_void,
    library: Option<Arc<Library>>,
    is_dropped: bool,
}

impl Drop for NativeGeneralPlugin {
    fn drop(&mut self) {
        if self.is_dropped {
            return;
        }

        if let Some(library) = &self.library {
            log::info!(
                "Deleting native general plugin: {} [{}]",
                self.name(),
                self.path().display()
            );
            unsafe {
                ((*library.vtable).plugin_destroy)(self.plugin_pointer);
            }
        } else {
            log::error!(
                "Couldn't drop general plugin because the library isn't loaded: {} [{}]",
                self.name(),
                self.path().display()
            );
        }
    }
}

impl NativeGeneralPlugin {
    pub fn tick(&mut self) -> Result<()> {
        if let Some(library) = &self.library {
            unsafe {
                ((*library.vtable).tick)(self.plugin_pointer);
            }
        } else {
            log::error!(
                "Couldn't tick general plugin: {:?} [{}]",
                self.name(),
                self.path.display()
            );
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        if let Some(library) = &self.library {
            unsafe {
                let cstr = CStr::from_ptr(((*library.vtable).name)(self.plugin_pointer));
                cstr.to_str().unwrap_or("UTF-8 ERROR")
            }
        } else {
            log::error!(
                "Couldn't get the name of general plugin: {}",
                self.path.display()
            );
            "UNKNOWN"
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }
}
