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
    lua_leds_buffer: Vec<Color>,
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
            lua_leds_buffer: vec![],
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

        let resize_fn: Function = match self.lua.globals().get("Resize_Colors") {
            Ok(resize_fn) => Ok(resize_fn),
            Err(_) => Err(LuaEffectRuntimeError::MissingFrameworkImport),
        }?;
        resize_fn
            .call::<_, Value>(leds.len())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let tick_fn: Function = match self.lua.globals().get("Tick") {
            Ok(tick_fn) => Ok(tick_fn),
            Err(_) => Err(LuaEffectRuntimeError::MissingTickFunction),
        }?;

        tick_fn
            .call::<_, ()>(())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let set_colors_fn: Function = match self.lua.globals().get("Set_colors") {
            Ok(set_colors_fn) => Ok(set_colors_fn),
            Err(_) => Err(LuaEffectRuntimeError::MissingFrameworkImport),
        }?;

        set_colors_fn
            .call::<_, ()>(())
            .map_err(LuaEffectRuntimeError::Lua)?;

        let data: mlua::String = self
            .lua
            .globals()
            .get("Colors_bin")
            .map_err(LuaEffectRuntimeError::Lua)?;
        let data = data.as_bytes();
        if self.lua_leds_buffer.len() != leds.len() {
            self.lua_leds_buffer.resize(leds.len(), Color::default());
        }
        if leds.len() * 3 != data.len() {
            return Err(LuaEffectRuntimeError::WrongColorsLen);
        }
        let dst = self.lua_leds_buffer.as_mut_ptr() as *mut u8;
        let src = data.as_ptr();
        // SAFETY: This code is safe and sound because what we are copying to is the `Color` struct
        // which has a C layout because of the #[repr(C)] directive.
        // Also, the memcopy here is garanteed to not create a buffer overflow because of the
        // bounds check earlier, and the memory will never be overlapping since we point to
        // different variables, have no overflow, and those variable have differents adresses in
        // memory. This code will remain safe as long as the `Color` struct's layout does not
        // change.
        // The usage of safety here is justified by the amount of performance gained when copying
        // data from our Lua script (~4X overall speed increase) without the need to go through
        // complicated seralization.
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst, data.len());
        }
        leds.copy_from_slice(self.lua_leds_buffer.as_slice());

        Ok(())
    }

    fn get_lua_effect(filename: &str) -> Result<(Lua, String, JSONSchema), LuaEffectLoadError> {
        let lua_src = fs::read_to_string(filename).map_err(LuaEffectLoadError::File)?;
        let lua = Lua::new();
        lua.load(&lua_src).exec().map_err(LuaEffectLoadError::Lua)?;
        let schema = Self::get_lua_schema(&lua)?;
        let compiled_schema = match JSONSchema::compile(&schema) {
            Ok(value) => Ok(value),
            Err(_) => Err(LuaEffectLoadError::Effect(
                InvalidEffectError::InvalidSchema,
            )),
        }?;

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
