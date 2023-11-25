use super::effects::{
    lua::LuaEffectSettings, moody::MoodySettings, native::NativeEffectSettings,
    raindrop::RaindropSettings,
};

#[derive(Debug)]
pub enum Settings {
    Lua(LuaEffectSettings),
    Native(NativeEffectSettings),
    Moody(MoodySettings),
    Raindrop(RaindropSettings),
}
