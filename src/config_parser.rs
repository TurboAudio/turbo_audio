use config::{Config, ConfigError, ValueKind};

use crate::pipewire_listener::{PortConnections, StreamConnections};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TurboAudioConfig {
    pub device_name: Option<String>,
    pub jack: bool,
    pub sample_rate: i64,
    pub stream_connections: Vec<StreamConnections>,
}

impl TurboAudioConfig {
    fn new() -> Self {
        Self {
            device_name: None,
            jack: false,
            sample_rate: 48000,
            stream_connections: vec![],
        }
    }
}

pub fn parse_config(config_name: &str) -> anyhow::Result<TurboAudioConfig> {
    let mut config = TurboAudioConfig::new();

    let settings = match Config::builder()
        .add_source(config::File::with_name(config_name))
        .build()
    {
        Ok(settings) => settings,
        Err(error) => {
            println!("{:?}", error);
            println!("Failed to get settings file. Default values used");
            return Ok(config);
        }
    };

    read_optional_variable(&mut config.jack, settings.get_bool("jack"))?;
    read_optional_variable(&mut config.sample_rate, settings.get_int("sample_rate"))?;

    let mut device_name = "Default Input Device".to_string();
    if read_optional_variable(&mut device_name, settings.get_string("device_name"))? {
        // Only set the device name if device name is in config
        config.device_name = Some(device_name);
    }

    let connections = settings.get_array("connections")?;
    for connection in connections {
        let connection = connection.into_table()?;
        let output_stream = connection
            .get("output_stream")
            .ok_or_else(|| anyhow::anyhow!("output_stream field missing from connection"))?
            .clone()
            .into_string()?;
        let input_stream = connection
            .get("input_stream")
            .ok_or_else(|| anyhow::anyhow!("input_stream field missing from connection"))?
            .clone()
            .into_string()?;
        let port_connections = connection
            .get("port_connections")
            .ok_or_else(|| anyhow::anyhow!("port_connections field missing from connection"))?;

        match &port_connections.kind {
            ValueKind::String(port_connections) => {
                if port_connections == "AllInOrder" {
                    config.stream_connections.push(StreamConnections {
                        output_stream,
                        input_stream,
                        port_connections: PortConnections::AllInOrder,
                    });
                } else {
                    anyhow::bail!("Invalid port connections: {}", port_connections);
                }
            }
            ValueKind::Array(port_connections) => {
                let mut connections_vec = Vec::new();
                for port_connection in port_connections {
                    let port_connection = port_connection.clone().into_table()?;
                    let out_port = port_connection
                        .get("out")
                        .ok_or_else(|| anyhow::anyhow!("port_connection doesn't have an out port"))?
                        .clone()
                        .into_string()?;
                    let in_port = port_connection
                        .get("in")
                        .ok_or_else(|| anyhow::anyhow!("port_connection doesn't have an in port"))?
                        .clone()
                        .into_string()?;
                    connections_vec.push((out_port, in_port));
                }
                config.stream_connections.push(StreamConnections {
                    output_stream,
                    input_stream,
                    port_connections: PortConnections::Only(connections_vec),
                });
            }
            type_kind => {
                anyhow::bail!("Invalid port connection type: {}", type_kind);
            }
        }
    }

    Ok(config)
}

fn read_optional_variable<T: Clone>(
    variable: &mut T,
    config_value: Result<T, ConfigError>,
) -> Result<bool, ConfigError> {
    match config_value {
        Ok(config_value) => {
            *variable = config_value;
            return Ok(true);
        }
        Err(config_error) => match config_error {
            ConfigError::NotFound(_) => {}
            other_error => {
                return Err(other_error);
            }
        },
    };
    Ok(false)
}
