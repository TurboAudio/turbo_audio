use crate::pipewire_listener::{PortConnections, StreamConnections};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurboAudioConfig {
    pub device_name: Option<String>,
    pub jack: bool,
    pub sample_rate: u32,
    pub stream_connections: Vec<StreamConnections>,
}
