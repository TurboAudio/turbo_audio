use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable, Deserialize, Serialize)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
