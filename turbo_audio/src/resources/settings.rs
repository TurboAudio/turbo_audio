use super::effects::{lua::LuaEffectSettings, native::NativeEffectSettings};

#[derive(Debug)]
pub enum Settings {
    Lua(LuaEffectSettings),
    Native(NativeEffectSettings),
}
