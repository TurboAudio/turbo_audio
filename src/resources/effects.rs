use self::{moody::Moody, raindrop::Raindrops};

pub mod moody;
pub mod raindrop;


pub enum Effect {
    Moody(Moody),
    Raindrop(Raindrops),
}
