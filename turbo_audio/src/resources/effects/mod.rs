use self::{lua::LuaEffect, native::NativeEffect};

pub mod lua;
pub mod native;

#[derive(Debug)]
pub enum Effect {
    Lua(LuaEffect),
    Native(NativeEffect),
}
