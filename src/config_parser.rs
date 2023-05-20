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

use std::{net::{SocketAddrV4, Ipv4Addr}, str::FromStr};

use crate::{
    audio_processing::AudioSignalProcessor,
    controller::Controller,
    resources::{
        color::Color,
        effects::{
            lua::{LuaEffect, LuaEffectSettings},
            moody::{Moody, MoodySettings},
            raindrop::{RaindropSettings, Raindrops, RaindropState},
            Effect,
        },
        settings::Settings, ledstrip::LedStrip,
    }, connections::{usb::UsbConnection, Connection, tcp::TcpConnection},
};
use config::{Config, ConfigError, Value};

pub fn parse_config(
    config_file_name: &str,
    audio_processor: &AudioSignalProcessor,
) -> anyhow::Result<Controller> {
    let settings = match Config::builder()
        .add_source(config::File::with_name(config_file_name))
        .build()
    {
        Ok(settings) => settings,
        Err(error) => {
            log::error!("{:?}", error);
            log::warn!("Failed to get settings file. Default values used");
            return Ok(Controller::default());
        }
    };

    // "effect_settings": [],
    // "effects": [],
    // "device_connections": [],
    // "ledstrips": []
    let mut controller = Controller::new();
    for effect_setting in settings.get_array("effect_settings")? {
        let effect_setting = effect_setting.into_table()?;
        let setting_id = read_variable(&effect_setting.get("id"))?.into_uint()? as usize;
        let setting_type = read_variable(&effect_setting.get("type"))?.into_string()?;

        match setting_type.as_str() {
            "Moody" => {
                let r = read_variable(&effect_setting.get("r"))?.into_uint()? as u8;
                let g = read_variable(&effect_setting.get("g"))?.into_uint()? as u8;
                let b = read_variable(&effect_setting.get("b"))?.into_uint()? as u8;
                let moody_setting = Settings::Moody(MoodySettings {
                    color: Color { r, g, b },
                });
                controller.add_settings(setting_id, moody_setting);
            }
            "Lua" => {
                // TODO: Implement LuaSettings parsing
                let lua_settings = serde_json::json!("{}");
                controller.add_settings(
                    setting_id,
                    Settings::Lua(LuaEffectSettings {
                        settings: lua_settings,
                    }),
                );
            }
            "Raindrop" => {
                let rain_speed = read_variable(&effect_setting.get("r"))?.into_int()? as i32;
                let drop_rate = read_variable(&effect_setting.get("r"))?.into_float()?;
                let rainbow_settings = RaindropSettings {
                    rain_speed,
                    drop_rate,
                };
                controller.add_settings(setting_id, Settings::Raindrop(rainbow_settings));
            }
            _ => continue,
        }
    }

    for effect in settings.get_array("effects")? {
        let effect = effect.into_table()?;
        let effect_id = read_variable(&effect.get("effect_id"))?.into_uint()? as usize;
        let setting_id = read_variable(&effect.get("setting_id"))?.into_uint()? as usize;
        let effect_type = read_variable(&effect.get("type"))?.into_string()?;
        match effect_type.as_str() {
            "Moody" => {
                let effect = Effect::Moody(Moody {});
                if controller.link_effect_to_settings(effect_id, setting_id) {
                    controller.add_effect(effect_id, effect);
                    log::info!("Loading Moody effect");
                }
            }
            "Lua" => {
                let lua_file_name = read_variable(&effect.get("file_name"))?.into_string()?;
                match LuaEffect::new(&lua_file_name, audio_processor) {
                    Ok(effect) => {
                        if controller.link_effect_to_settings(effect_id, setting_id) {
                            controller.add_effect(effect_id, Effect::Lua(effect));
                            log::info!("Loading lua effect {lua_file_name}");
                        }
                    },
                    Err(e) => {
                        log::error!("{e:?}");
                        continue;
                    }
                };
            }
            "Raindrop" => {
                let effect = Effect::Raindrop(Raindrops { state: RaindropState{riples: Vec::default()} });
                if controller.link_effect_to_settings(effect_id, setting_id) {
                    controller.add_effect(effect_id, effect);
                }
            }
            _ => {}
        }
    }

    for connection in settings.get_array("device_connections")? {
        let connection = connection.into_table()?;
        let connection_id = read_variable(&connection.get("id"))?.into_uint()? as usize;
        let connection_type = read_variable(&connection.get("type"))?.into_string()?;
        match connection_type.as_str() {
            "Tcp" => {
                let ip = read_variable(&connection.get("ip"))?.into_string()?;
                let ip = Ipv4Addr::from_str(&ip)?;
                let port = read_variable(&connection.get("port"))?.into_uint()? as u16;
                let ip = std::net::SocketAddr::V4(SocketAddrV4::new(ip, port));
                let connection = Connection::Tcp(TcpConnection::new(ip));
                log::info!("Adding Tcp connection {ip:?}");
                controller.add_connection(connection_id, connection);
            },
            "Usb" => {
                let connection = Connection::Usb(UsbConnection{});
                controller.add_connection(connection_id, connection);
            },
            _ => continue
        }
    }

    for ledstip in settings.get_array("ledstrips")? {
        let ledstrip_config = ledstip.into_table()?;
        let ledstrip_id = read_variable(&ledstrip_config.get("ledstrip_id"))?.into_uint()? as usize;
        let connection_id = read_variable(&ledstrip_config.get("connection_id"))?.into_uint()? as usize;
        let ledstrip_size = read_variable(&ledstrip_config.get("size"))?.into_uint()? as usize;
        let mut ledstrip = LedStrip::default();
        ledstrip.set_led_count(ledstrip_size);
        let mut is_valid_config = true;
        for effect in read_variable(&ledstrip_config.get("effects"))?.into_array()? {
            let effect = effect.into_table()?;
            let effect_id = read_variable(&effect.get("effect_id"))?.into_uint()? as usize;
            let effect_size = read_variable(&effect.get("effect_size"))?.into_uint()? as usize;
            if !ledstrip.add_effect(effect_id, effect_size) {
                is_valid_config = false;
                break;
            }
        }
        if is_valid_config && controller.link_led_strip_to_connection(ledstrip_id, connection_id) {
            controller.add_led_strip(ledstrip_id, ledstrip);
            log::info!("Added ledstrip {ledstrip_id} and linked with connection {connection_id}");
        }
    }

    Ok(controller)
}

fn read_variable(config_value: &Option<&Value>) -> Result<Value, ConfigError> {
    match config_value {
        Some(value) => Ok(Clone::clone(value)),
        None => Err(ConfigError::NotFound(
            "Unable to find config value.".to_string(),
        )),
    }
}

