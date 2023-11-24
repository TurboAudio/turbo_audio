use std::path::PathBuf;

use crate::audio::pipewire_listener::StreamConnections;
use crate::resources::color::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum EffectConfigType {
    Lua(String),
    Moody(),
    Raindrop(),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SettingsConfigType {
    Lua(serde_json::Value),
    Moody(Color),
    Raindrop(),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConnectionConfigType {
    Tcp(std::net::SocketAddr),
    Usb(),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EffectConfig {
    pub effect_id: usize,
    pub settings_id: usize,
    pub effect: EffectConfigType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EffectSettingConfig {
    pub id: usize,
    pub setting: SettingsConfigType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LedstripEffectConfig {
    pub effect_id: usize,
    pub effect_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LedstripConfig {
    pub id: usize,
    pub connection_id: usize,
    pub size: usize,
    pub effects: Vec<LedstripEffectConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub connection: ConnectionConfigType,
    pub id: usize,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TurboAudioConfig {
    pub lua_effects_folder: PathBuf,
    pub device_name: Option<String>,
    pub sample_rate: u32,
    pub stream_connections: Vec<StreamConnections>,
    pub effect_settings: Vec<EffectSettingConfig>,
    pub effects: Vec<EffectConfig>,
    pub devices: Vec<DeviceConfig>,
    pub ledstrips: Vec<LedstripConfig>,
}
