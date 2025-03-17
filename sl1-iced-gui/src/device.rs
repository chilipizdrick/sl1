use crate::types::{Port, PresetId};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    wifi_settings: DeviceWifiSettings,
    preset_settings: Vec<PresetSettings>,
    current_preset_id: PresetId,
    is_on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWifiSettings {
    ssid: String,
    password: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PresetSettings {
    brightness: u8,
    speed: u8,
    scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    id: PresetId,
    name: String,
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Preset {
    pub fn new(id: PresetId, name: String) -> Self {
        Self { id, name }
    }
}

#[derive(Debug)]
pub struct Device {
    ip: IpAddr,
    port: Port,
}

impl Default for Device {
    fn default() -> Self {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));
        let port = Port::new(8080).unwrap();
        Self { ip, port }
    }
}

impl Device {
    pub fn new(ip: IpAddr, port: Port) -> Self {
        Self { ip, port }
    }

    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    pub fn port(&self) -> Port {
        self.port
    }
}
