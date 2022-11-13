use super::effects::{moody::MoodySettings, raindrop::RaindropSettings};

pub enum Settings {
    Moody(MoodySettings),
    Raindrop(RaindropSettings),
}

impl Settings {
    pub fn moody(&self) -> &MoodySettings {
        match self {
            Settings::Moody(moody) => moody,
            _ => unreachable!(),
        }
    }

    pub fn mut_moody(&mut self) -> &mut MoodySettings {
        match self {
            Settings::Moody(moody) => moody,
            _ => unreachable!(),
        }
    }
    pub fn raindrop(&self) -> &RaindropSettings {
        match self {
            Settings::Raindrop(raindrop) => raindrop,
            _ => unreachable!(),
        }
    }
}
