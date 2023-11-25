use self::{lua::LuaEffect, moody::Moody, native::NativeEffect, raindrop::Raindrops};

pub mod lua;
pub mod moody;
pub mod native;
pub mod raindrop;

#[derive(Debug)]
pub enum Effect {
    Lua(LuaEffect),
    Native(NativeEffect),
    Moody(Moody),
    Raindrop(Raindrops),
}
