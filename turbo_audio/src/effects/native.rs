use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

use turbo_plugin::VTable;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error when loading native library: {0}")]
    LoadError(#[from] libloading::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct NativeEffectManager {
    libraries: HashMap<PathBuf, Library>,
}

impl Drop for NativeEffectManager {
    fn drop(&mut self) {
        log::info!("Dropping Native Effects Mgr");
    }
}

struct Library {
    _library: libloading::Library,
    vtable: *const VTable,
}

impl Drop for Library {
    fn drop(&mut self) {
        log::info!("Dropping library");
    }
}

impl NativeEffectManager {
    pub fn create_effect(&mut self, effect_path: impl AsRef<Path>) -> Result<NativeEffect> {
        let path = std::fs::canonicalize(&effect_path).unwrap();

        let library = match self.libraries.entry(path) {
            std::collections::hash_map::Entry::Occupied(occupied) => occupied.into_mut(),
            std::collections::hash_map::Entry::Vacant(vacant) => {
                unsafe {
                    // "../effects/rust/soin/target/debug/libsoin.so",
                    let library = libloading::Library::new(vacant.key())?;

                    let vtable_fn = library
                        .get::<extern "C" fn() -> *const std::ffi::c_void>(b"_plugin_vtable")?;

                    let vtable = vtable_fn() as *const turbo_plugin::VTable;

                    ((*vtable).load)();

                    vacant.insert(Library {
                        _library: library,
                        vtable,
                    })
                }
            }
        };

        let plugin = unsafe { ((*library.vtable).plugin_create)() };
        Ok(NativeEffect {
            pointer: plugin,
            vtable: library.vtable,
        })
    }
}

#[derive(Debug)]
pub struct NativeEffectSettings {}

#[derive(Debug)]
pub struct NativeEffect {
    pointer: *mut std::ffi::c_void,
    vtable: *const VTable,
}

impl Drop for NativeEffect {
    fn drop(&mut self) {
        log::info!("Dropping native effect");
        unsafe {
            ((*self.vtable).plugin_destroy)(self.pointer);
        }
    }
}

impl NativeEffect {
    pub fn tick(&mut self) -> Result<()> {
        unsafe { ((*self.vtable).tick)(self.pointer) }
        Ok(())
    }
}
