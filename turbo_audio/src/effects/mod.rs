use self::{
    lua::{LuaEffect, LuaEffectSettings},
    native::{NativeEffect, NativeEffectSettings},
};

pub mod lua;
pub mod native;

#[derive(Debug)]
pub enum Effect {
    Lua(LuaEffect),
    Native(NativeEffect),
}

#[derive(Debug)]
pub enum Settings {
    Lua(LuaEffectSettings),
    Native(NativeEffectSettings),
}
