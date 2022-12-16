use crate::resources::color::Color;
use jsonschema::JSONSchema;
use mlua::{Error, Function, Lua, LuaSerdeExt, Table, Value};
use std::fs;

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

#[allow(unused)]
#[derive(Debug)]
pub struct LuaEffect {
    lua: Lua,
    json_schema: String,
    compiled_json_schema: JSONSchema,
}

#[derive(Clone, Debug)]
pub struct LuaEffectSettings {
    pub settings: serde_json::Value,
}

impl LuaEffect {
    pub fn new(filename: &str) -> Result<Self, LuaEffectLoadError> {
        let (lua, json_schema, compiled_json_schema) = Self::get_lua_effect(filename)?;
        Ok(LuaEffect {
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

    fn get_lua_effect(filename: &str) -> Result<(Lua, String, JSONSchema), LuaEffectLoadError> {
        let lua_src = fs::read_to_string(filename).map_err(LuaEffectLoadError::File)?;
        let lua = Lua::new();
        lua.load(&lua_src).exec().map_err(LuaEffectLoadError::Lua)?;
        let schema = Self::get_lua_schema(&lua)?;
        let compiled_schema = JSONSchema::compile(&schema)
            .map_err(|_| LuaEffectLoadError::Effect(InvalidEffectError::InvalidSchema))?;

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
