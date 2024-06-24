use std::path::Path;

use dasp_ring_buffer::SliceMut;

use crate::audio::audio_processing::AudioSignalProcessor;

use super::native::{NativeGeneralPlugin, NativeGeneralPluginManager};

pub struct GeneralPluginManager {
    native_general_plugin_manager: NativeGeneralPluginManager,
    plugins: Vec<NativeGeneralPlugin>,
}

impl GeneralPluginManager {
    pub fn new(audio_processor: &AudioSignalProcessor) -> Self {
        Self {
            native_general_plugin_manager: NativeGeneralPluginManager::new(audio_processor),
            plugins: Default::default(),
        }
    }

    pub fn load_native_plugin(&mut self, plugin_path: impl AsRef<Path>) {
        match self
            .native_general_plugin_manager
            .create_plugin(&plugin_path)
        {
            Ok(plugin) => {
                self.plugins.push(plugin);
                log::info!("Created general plugin: {}", plugin_path.as_ref().display());
            }
            Err(error) => {
                log::error!(
                    "Can't create plugin: {}, {error}",
                    plugin_path.as_ref().display()
                );
            }
        }
    }

    pub fn tick_all(&mut self) {
        for plugin in self.plugins.slice_mut() {
            if let Err(e) = plugin.tick() {
                log::error!("Error while ticking {}: {e}", plugin.path().display());
            }
        }
    }
}
