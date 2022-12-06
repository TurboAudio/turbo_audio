use self::{moody::Moody, raindrop::Raindrops};

pub mod moody;
pub mod raindrop;


pub enum Effect {
    Moody(Moody),
    Raindrop(Raindrops),
}

impl Effect {
    pub fn settings_id(&self) -> &i32 {
        match self {
            Effect::Moody(moody) => &moody.settings_id,
            Effect::Raindrop(raindrop) => &raindrop.settings_id,
        }
    }
}
