use super::effects::{moody::MoodySettings, raindrop::RaindropSettings};

pub enum Settings {
    Moody(MoodySettings),
    Raindrop(RaindropSettings),
}

