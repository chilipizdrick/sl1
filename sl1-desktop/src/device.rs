use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};

pub type PresetId = u8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    wifi_settings: DeviceWifiSettings,
    preset_settings: Vec<PresetSettings>,
    current_preset_id: PresetId,
    is_on: bool,
}

impl DeviceSettings {
    #[allow(unused)]
    pub fn wifi_settings(&self) -> &DeviceWifiSettings {
        &self.wifi_settings
    }

    pub fn preset_settings(&self) -> &[PresetSettings] {
        &self.preset_settings
    }

    pub fn current_preset_id(&self) -> u8 {
        self.current_preset_id
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWifiSettings {
    ssid: String,
    password: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PresetSettings {
    #[serde(rename = "b")]
    brightness: u8,
    #[serde(rename = "sp")]
    speed: u8,
    #[serde(rename = "sc")]
    scale: u8,
}

impl PresetSettings {
    pub fn brightness(&self) -> u8 {
        self.brightness
    }

    pub fn speed(&self) -> u8 {
        self.speed
    }

    pub fn scale(&self) -> u8 {
        self.scale
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    id: PresetId,
    name: String,
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.id, self.name)
    }
}

impl Preset {
    pub fn id(&self) -> u8 {
        self.id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    ip_addr: IpAddr,
    port: u16,
}

impl Device {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self { ip_addr: ip, port }
    }

    pub fn ip(&self) -> IpAddr {
        self.ip_addr
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip_addr, self.port)
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip_addr, self.port)
    }
}
