use self::{
    lua::{LuaEffect, LuaEffectSettings},
    native::{NativeEffect, NativeEffectSettings},
    python::{PythonEffect, PythonEffectSettings},
};

pub mod lua;
pub mod native;
pub mod python;

#[derive(Debug)]
pub enum Effect {
    Lua(LuaEffect),
    Native(NativeEffect),
    Python(PythonEffect),
}

#[derive(Debug)]
pub enum EffectSettings {
    Lua(LuaEffectSettings),
    Native(NativeEffectSettings),
    Python(PythonEffectSettings),
}
