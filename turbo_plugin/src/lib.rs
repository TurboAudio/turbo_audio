use std::any::Any;

pub trait Plugin: Any + Send + Sync {
    /// Get a name describing the `Plugin`.
    fn name(&self) -> *const std::ffi::c_char;

    /// Tick fn
    fn tick(&self);

    /// A callback fired immediately after the plugin is loaded. Usually used
    /// for initialization.
    fn load();

    /// A callback fired immediately before the plugin is unloaded. Use this if
    /// you need to do any cleanup.
    fn unload();
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct VTable {
    /// Function that returns a pointer to a heap allocated plugin
    pub plugin_create: extern "C" fn() -> *mut std::ffi::c_void,

    /// Function that destroys a heap allocated plugin
    pub plugin_destroy: extern "C" fn(*mut std::ffi::c_void),

    /// Function that returns the name of the plugin
    pub name: extern "C" fn(*const std::ffi::c_void) -> *const std::ffi::c_char,

    /// Function that ticks the plugin
    pub tick: extern "C" fn(*const std::ffi::c_void),

    /// Function that gets called when the shared library gets loaded
    /// Useful for making initialization that is shared between plugin instances
    pub load: extern "C" fn(),

    /// Function that gets called when the shared library gets unloaded
    /// Useful for cleaning up anything that was initialized during the `on_load` function
    pub unload: extern "C" fn(),
}

/// Declare a plugin type and its constructor.
///
/// # Notes
///
/// This works by automatically generating an `extern "C"` function with a
/// pre-defined signature and symbol name. Therefore you will only be able to
/// declare one plugin per library.
#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut dyn turbo_plugin::Plugin {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<$crate::Plugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}
