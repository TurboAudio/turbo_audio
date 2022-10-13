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
pub struct TurboAudioConfigBuilder(Option<String>);

impl TurboAudioConfig {
    pub fn builder() -> TurboAudioConfigBuilder {
        TurboAudioConfigBuilder(None)
    }

    pub fn default() -> Self {
        Self {
            device_name: None,
            jack: false,
            sample_rate: 48000,
            stream_connections: Vec::new(),
        }
    }
}

impl TurboAudioConfigBuilder {
    pub fn add_source(mut self, config_name: &str) -> TurboAudioConfigBuilder {
        self.0 = config_name.to_owned().into();
        self
    }

    pub fn build(self) -> anyhow::Result<TurboAudioConfig> {
        let config_name = self
            .0
            .ok_or_else(|| anyhow::anyhow!("source was not added"))?;
        let settings = match Config::builder()
            .add_source(config::File::with_name(&config_name))
            .build()
        {
            Ok(settings) => settings,
            Err(error) => {
                println!("{:?}", error);
                println!("Failed to get settings file. Default values used");
                return Ok(TurboAudioConfig::default());
            }
        };
        

        let mut stream_connections = Vec::new();
        for connection in settings.get_array("connections")? {
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
                .ok_or_else(|| anyhow::anyhow!("port_connections field missing from connection"))?
                .clone();

            match port_connections.kind {
                ValueKind::String(port_connections) => {
                    if port_connections == "AllInOrder" {
                        stream_connections.push(StreamConnections {
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
                        let port_connection = port_connection.into_table()?;
                        let out_port = port_connection
                            .get("out")
                            .ok_or_else(|| {
                                anyhow::anyhow!("port_connection doesn't have an out port")
                            })?
                            .clone()
                            .into_string()?;
                        let in_port = port_connection
                            .get("in")
                            .ok_or_else(|| {
                                anyhow::anyhow!("port_connection doesn't have an in port")
                            })?
                            .clone()
                            .into_string()?;

                        connections_vec.push((out_port, in_port));
                    }
                    stream_connections.push(StreamConnections {
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

        let jack = read_optional_variable(settings.get_bool("jack"))?.unwrap_or(false);
        let sample_rate = read_optional_variable(settings.get_int("sample_rate"))?.unwrap_or(48000);
        let device_name = read_optional_variable(settings.get_string("device_name"))?
            .unwrap_or_else(|| "Default Input Device".to_string());

        Ok(TurboAudioConfig {
            device_name: device_name.into(),
            jack,
            sample_rate,
            stream_connections,
        })
    }
}

fn read_optional_variable<T: Clone>(
    config_value: Result<T, ConfigError>,
) -> Result<Option<T>, ConfigError> {
    match config_value {
        Ok(config_value) => Ok(config_value.into()),
        Err(config_error) => match config_error {
            ConfigError::NotFound(_) => Ok(None),
            other_error => Err(other_error),
        },
    }
}
