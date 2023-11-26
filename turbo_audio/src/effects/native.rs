use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use libloading::os::unix::{RTLD_LOCAL, RTLD_NOW};
use turbo_plugin::VTable;

use thiserror::Error;

use super::Effect;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error when loading native library: {0}")]
    LoadError(#[from] libloading::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct NativeEffectsManager {
    libraries: HashMap<PathBuf, Arc<Library>>,
}

#[derive(Debug)]
struct Library {
    library: Option<libloading::Library>,
    vtable: *const VTable,
    id: u64,
}

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
    pub fn create_effect(&mut self, effect_path: impl AsRef<Path>) -> Result<Effect> {
        let path = std::fs::canonicalize(&effect_path).unwrap();

        let library = match self.libraries.entry(path) {
            std::collections::hash_map::Entry::Occupied(occupied) => occupied.into_mut(),
            std::collections::hash_map::Entry::Vacant(vacant) => unsafe {
                let library =
                    libloading::os::unix::Library::open(Some(vacant.key()), RTLD_NOW | RTLD_LOCAL)?;

                let vtable_fn =
                    library.get::<extern "C" fn() -> *const std::ffi::c_void>(b"_plugin_vtable")?;

                let vtable = vtable_fn() as *const turbo_plugin::VTable;

                ((*vtable).load)();

                vacant.insert(Arc::new(Library {
                    library: Some(library.into()),
                    vtable,
                    id: rand::random(),
                }))
            },
        };

        log::error!("Creating effect from library: {}", library.id);
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
        unsafe {
            log::info!("Reloading library: {}", path.as_ref().display());
            let Ok(library) =
                libloading::os::unix::Library::open(Some(path.as_ref()), RTLD_NOW | RTLD_LOCAL)
            else {
                log::error!("asdfs");
                return;
            };

            let Ok(vtable_fn) =
                library.get::<extern "C" fn() -> *const std::ffi::c_void>(b"_plugin_vtable")
            else {
                log::error!("asd;lfkjasdf");
                return;
            };

            let vtable = vtable_fn() as *const turbo_plugin::VTable;

            ((*vtable).load)();

            // todo impl send & sync
            self.libraries.insert(
                path.as_ref().to_owned(),
                Arc::new(Library {
                    library: Some(library.into()),
                    vtable,
                    id: rand::random(),
                }),
            );
        }
    }

    pub fn pre_reload_effect(&mut self, effect: &mut NativeEffect) {
        if let Some(library) = &effect.library {
            unsafe {
                ((*library.vtable).plugin_destroy)(effect.pointer);
            }
            effect.is_dropped = true;
        }
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
    pub fn tick(&mut self) -> Result<()> {
        if let Some(library) = &self.library {
            unsafe {
                ((*library.vtable).tick)(self.pointer);
            }
        }
        Ok(())
    }
}
