use super::effects::{lua::LuaEffectSettings, moody::MoodySettings, raindrop::RaindropSettings};

#[derive(Debug)]
pub enum Settings {
    Lua(LuaEffectSettings),
    Moody(MoodySettings),
    Raindrop(RaindropSettings),
}
