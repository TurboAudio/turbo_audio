use self::{lua::LuaEffect, moody::Moody, raindrop::Raindrops};

pub mod lua;
pub mod moody;
pub mod raindrop;

#[derive(Debug)]
pub enum Effect {
    Lua(LuaEffect),
    Moody(Moody),
    Raindrop(Raindrops),
}
