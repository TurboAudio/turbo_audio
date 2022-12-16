use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
impl Color {
    pub fn new() -> Color {
        BLACK
    }

    pub fn add(&mut self, rhs: &Color) {
        self.r = self.r.saturating_add(rhs.r);
        self.g = self.g.saturating_add(rhs.g);
        self.b = self.b.saturating_add(rhs.b);
    }

    pub fn to_bytes(self) -> Vec<u8> {
        vec![self.r, self.g, self.b]
    }
}
