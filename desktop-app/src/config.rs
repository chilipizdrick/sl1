use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::{IpAddr, Ipv4Addr},
    path::Path,
};

use crate::{
    device::{Device, Preset},
    error::{Error, Result},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    device: Device,
    preset_info: Vec<Preset>,
}

impl Config {
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_string = toml::to_string(&self).map_err(|_| Error::Serialization)?;
        fs::write(config_path, config_string).map_err(|_| Error::Fs)
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        if !config_path.exists() {
            return Err(Error::MissingConfig);
        }
        let config_string = std::fs::read_to_string(config_path).map_err(|_| Error::Fs)?;
        toml::from_str(&config_string).map_err(|_| Error::Deserialization)
    }

    fn config_path() -> Result<Box<Path>> {
        let base_dirs = BaseDirs::new().ok_or(Error::Fs)?;
        let path = base_dirs.config_dir().join("sl1/");
        if !path.exists() {
            fs::create_dir_all(&path).map_err(|_| Error::Fs)?;
        }
        Ok(path.join("config.toml").into())
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn set_device(&mut self, device: Device) {
        self.device = device;
    }

    pub fn preset_info(&self) -> &[Preset] {
        &self.preset_info
    }

    pub fn set_preset_info(&mut self, preset_info: Vec<Preset>) {
        self.preset_info = preset_info;
    }
}

impl Default for Config {
    fn default() -> Self {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        let port = 8080;
        let device = Device::new(ip, port);
        let preset_info = Vec::new();
        Self {
            device,
            preset_info,
        }
    }
}
