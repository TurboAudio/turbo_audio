use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable, Deserialize, Serialize)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

impl Color {
    pub fn add(&mut self, rhs: &Color) {
        self.r = self.r.saturating_add(rhs.r);
        self.g = self.g.saturating_add(rhs.g);
        self.b = self.b.saturating_add(rhs.b);
    }
}
