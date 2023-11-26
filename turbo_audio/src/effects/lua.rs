use crate::{
    audio::{audio_processing::AudioSignalProcessor, audio_processing::FftResult},
    resources::color::Color,
};
use jsonschema::JSONSchema;
use mlua::{Error, Function, Lua, LuaSerdeExt, Table, Value};
use std::{
    fs,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use super::Effect;

#[derive(Debug)]
pub enum InvalidEffectError {
    MissingSchema,
    InvalidSchema,
}

#[derive(Debug)]
pub enum LuaEffectLoadError {
    File(std::io::Error),
    Lua(Error),
    Effect(InvalidEffectError),
}

#[derive(Debug)]
pub enum LuaEffectRuntimeError {
    Lua(Error),
    WrongColorsLen,
    MissingTickFunction,
    MissingFrameworkImport,
}

pub struct LuaEffectsManager {
    package_root: PathBuf,
}

impl LuaEffectsManager {
    pub fn new(package_root: impl AsRef<Path>) -> Self {
        Self {
            package_root: package_root.as_ref().to_owned(),
        }
    }

    pub fn create_effect(
        &mut self,
        effect_path: impl AsRef<Path>,
        audio_processor: &AudioSignalProcessor,
    ) -> Result<Effect, LuaEffectLoadError> {
        let effect = Effect::Lua(LuaEffect::new(
            &effect_path,
            &self.package_root,
            audio_processor,
        )?);
        Ok(effect)
    }

    pub fn on_file_changed(&mut self, _file: impl AsRef<Path>) {}

    pub fn reload_effect(
        &mut self,
        effect_to_reload: &mut LuaEffect,
        audio_processing: &AudioSignalProcessor,
    ) {
        let Ok(new_effect) =
            LuaEffect::new(&effect_to_reload.path, &self.package_root, audio_processing)
        else {
            log::error!("cringe");
            return;
        };

        let _ = std::mem::replace(effect_to_reload, new_effect);
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct LuaEffect {
    path: PathBuf,
    lua: Lua,
    json_schema: String,
    compiled_json_schema: JSONSchema,
}

#[derive(Clone, Debug)]
pub struct LuaEffectSettings {
    pub settings: serde_json::Value,
}

struct LuaFftResult {
    fft_result: Arc<RwLock<FftResult>>,
}

impl mlua::UserData for LuaFftResult {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(
            "get_average_amplitude",
            |_, this, (lower_frequency, upper_frequency): (f32, f32)| {
                let result = this
                    .fft_result
                    .read()
                    .unwrap()
                    .get_average_amplitude(lower_frequency, upper_frequency)
                    .unwrap_or_else(|| {
                        log::error!("Invalid frequencies: {lower_frequency} & {upper_frequency}");
                        0.0f32
                    });
                Ok(result)
            },
        );

        methods.add_method("get_frequency_amplitude", |_, this, frequency: f32| {
            let result = this
                .fft_result
                .read()
                .unwrap()
                .get_frequency_amplitude(frequency)
                .unwrap_or_else(|| {
                    log::error!("Invalid frequency: {frequency}");
                    0.0f32
                });
            Ok(result)
        });

        methods.add_method("get_max_frequency", |_, this, _: ()| {
            Ok(this.fft_result.read().unwrap().get_max_frequency())
        });
    }
}

impl LuaEffect {
    fn new(
        effect_path: impl AsRef<Path>,
        package_root: impl AsRef<Path>,
        audio_processor: &AudioSignalProcessor,
    ) -> Result<Self, LuaEffectLoadError> {
        log::info!("Loading lua effect: {}", effect_path.as_ref().display());
        let (lua, json_schema, compiled_json_schema) =
            Self::load_lua_effect(&effect_path, &package_root, audio_processor)?;
        Ok(Self {
            path: effect_path.as_ref().to_path_buf(),
            lua,
            json_schema,
            compiled_json_schema,
        })
    }

    pub fn tick(
        &mut self,
        leds: &mut [Color],
        settings: &LuaEffectSettings,
    ) -> Result<(), LuaEffectRuntimeError> {
        self.lua
            .globals()
            .set("settings", self.lua.to_value(&settings.settings).unwrap())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let resize_fn: Function = self
            .lua
            .globals()
            .get("Resize_Colors")
            .map_err(|_| LuaEffectRuntimeError::MissingFrameworkImport)?;

        resize_fn
            .call::<_, Value>(leds.len())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let tick_fn: Function = self
            .lua
            .globals()
            .get("Tick")
            .map_err(|_| LuaEffectRuntimeError::MissingTickFunction)?;

        tick_fn
            .call::<_, ()>(())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let set_colors_fn: Function = self
            .lua
            .globals()
            .get("Set_colors")
            .map_err(|_| LuaEffectRuntimeError::MissingFrameworkImport)?;

        set_colors_fn
            .call::<_, ()>(())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let data = self
            .lua
            .globals()
            .get::<_, mlua::String>("Colors_bin")
            .map_err(LuaEffectRuntimeError::Lua)?;
        let data = data.as_bytes();

        if leds.len() * 3 != data.len() {
            return Err(LuaEffectRuntimeError::WrongColorsLen);
        }

        leds.copy_from_slice(
            &data
                .chunks_exact(3)
                .map(|s| Color {
                    r: s[0],
                    g: s[1],
                    b: s[2],
                })
                .collect::<Vec<_>>(),
        );

        Ok(())
    }

    fn load_lua_effect(
        path: impl AsRef<Path>,
        package_path: impl AsRef<Path>,
        audio_processor: &AudioSignalProcessor,
    ) -> Result<(Lua, String, JSONSchema), LuaEffectLoadError> {
        let lua_src = fs::read_to_string(path).map_err(LuaEffectLoadError::File)?;
        let lua = Lua::new();

        {
            // Add our package path to lua's package path so that it can find the libraries
            let package = lua.globals().get::<_, mlua::Table>("package").unwrap();
            let path = package.get::<_, mlua::String>("path").unwrap();
            let mut new_str = Vec::from(path.as_bytes());
            new_str.extend_from_slice(b";");
            new_str.extend_from_slice(package_path.as_ref().as_os_str().as_bytes());
            if new_str.last() != Some(&b'/') {
                new_str.extend_from_slice(b"/");
            }
            new_str.extend_from_slice(b"?.lua");
            package
                .set("path", lua.create_string(&new_str).unwrap())
                .unwrap(); // Append the new search path to package.path

            lua.globals().set("package", package).unwrap(); // Update the package
        }

        lua.load(&lua_src).exec().map_err(LuaEffectLoadError::Lua)?;
        let schema = Self::get_lua_schema(&lua)?;
        let compiled_schema = JSONSchema::compile(&schema)
            .map_err(|_| LuaEffectLoadError::Effect(InvalidEffectError::InvalidSchema))?;

        lua.globals()
            .set(
                "Fft_Result",
                LuaFftResult {
                    fft_result: audio_processor.fft_result.clone(),
                },
            )
            .unwrap();

        Ok((lua, schema.to_string(), compiled_schema))
    }

    fn get_lua_schema(lua: &Lua) -> Result<serde_json::Value, LuaEffectLoadError> {
        let schema = lua
            .globals()
            .get::<_, Table>("SettingsSchema")
            .map_err(|_| LuaEffectLoadError::Effect(InvalidEffectError::MissingSchema))?;
        let schema = serde_json::json!(&schema);
        Ok(schema)
    }
}
