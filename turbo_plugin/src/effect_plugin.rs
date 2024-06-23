use crate::{audio_api, Color};
use std::any::Any;

pub trait NativeEffectPlugin: Any + Send + Sync {
    /// Get a name describing the `Plugin`.
    fn name(&self) -> *const std::ffi::c_char;

    /// Tick fn
    fn tick(&self, leds: &mut [Color]);

    /// A callback called immediately after the plugin is loaded. Usually used
    /// for initialization.
    fn load();

    /// A callback called immediately before the plugin is unloaded. Use this if
    /// you need to do any cleanup.
    fn unload();
}

#[macro_export]
macro_rules! make_native_effect_plugin {
    ($plugin:ty, $ctor:expr) => {
        #[no_mangle]
        extern "C" fn _plugin_vtable() -> *const std::ffi::c_void {
            extern "C" fn plugin_create() -> *mut std::ffi::c_void {
                let plugin = Box::new($ctor);
                Box::into_raw(plugin) as *mut _
            }

            extern "C" fn plugin_destroy(plugin: *mut std::ffi::c_void) {
                unsafe {
                    drop(Box::from_raw(plugin as *mut $plugin));
                }
            }

            extern "C" fn name(plugin: *const std::ffi::c_void) -> *const std::ffi::c_char {
                let plugin = unsafe { &*(plugin as *const $plugin) };
                plugin.name()
            }

            extern "C" fn tick(
                plugin: *const std::ffi::c_void,
                colors: *mut Color,
                len: std::ffi::c_ulong,
            ) {
                let plugin = unsafe { &*(plugin as *const $plugin) };
                let slice = unsafe { std::slice::from_raw_parts_mut(colors, len as _) };
                plugin.tick(slice);
            }

            extern "C" fn load(audio_api: turbo_plugin::audio_api::AudioApi) {
                turbo_plugin::audio_api::on_load(audio_api);
                <$plugin>::load();
            }

            extern "C" fn unload() {
                turbo_plugin::audio_api::free();
                <$plugin>::unload();
            }

            static VTABLE: turbo_plugin::effect_plugin::NativeEffectPluginVTable =
                turbo_plugin::effect_plugin::NativeEffectPluginVTable {
                    plugin_create,
                    plugin_destroy,
                    name,
                    tick,
                    load,
                    unload,
                };

            &VTABLE as *const turbo_plugin::effect_plugin::NativeEffectPluginVTable as *const _
        }
    };
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct NativeEffectPluginVTable {
    /// Function that returns a pointer to a heap allocated plugin
    pub plugin_create: extern "C" fn() -> *mut std::ffi::c_void,

    /// Function that destroys a heap allocated plugin
    pub plugin_destroy: extern "C" fn(*mut std::ffi::c_void),

    /// Function that returns the name of the plugin
    pub name: extern "C" fn(*const std::ffi::c_void) -> *const std::ffi::c_char,

    /// Function that ticks the plugin
    pub tick: extern "C" fn(*const std::ffi::c_void, *mut Color, std::ffi::c_ulong),

    /// Function that gets called when the shared library gets loaded
    /// Useful for making initialization that is shared between plugin instances
    pub load: extern "C" fn(audio_api::AudioApi),

    /// Function that gets called when the shared library gets unloaded
    /// Useful for cleaning up anything that was initialized during the `on_load` function
    pub unload: extern "C" fn(),
}
