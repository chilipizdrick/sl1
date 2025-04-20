use core::str::FromStr;
use embedded_storage::{ReadStorage, Storage};
use serde::{Deserialize, Serialize};

use crate::{
    Error, PRESET_COUNT, Result, SETTINGS_STORAGE_OFFSET, STORAGE, WIFI_PASSWORD, WIFI_SSID,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub wifi_settings: WifiSettings,
    pub preset_settings: [PresetSettings; PRESET_COUNT as usize],
    pub current_preset_id: PresetId,
    pub is_on: bool,
}

impl From<&[u8; core::mem::size_of::<Self>()]> for Settings {
    fn from(bytes: &[u8; core::mem::size_of::<Self>()]) -> Self {
        unsafe { core::mem::transmute(*bytes) }
    }
}

impl<'a> From<&'a Settings> for &'a [u8] {
    fn from(value: &'a Settings) -> Self {
        unsafe { as_u8_slice(value) }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wifi_settings: WifiSettings::default(),
            preset_settings: [PresetSettings::default(); PRESET_COUNT as usize],
            current_preset_id: PresetId::new_fallible(1).unwrap(),
            is_on: true,
        }
    }
}

impl Settings {
    pub async fn save(&self) -> Result<()> {
        STORAGE
            .get()
            .lock()
            .await
            .write(SETTINGS_STORAGE_OFFSET, self.into())
            .map_err(Error::StorageWriteError)?;
        Ok(())
    }

    pub async fn load() -> Result<Self> {
        let mut buf = [0u8; core::mem::size_of::<Settings>()];
        STORAGE
            .get()
            .lock()
            .await
            .read(SETTINGS_STORAGE_OFFSET, &mut buf)
            .map_err(Error::StorageReadError)?;
        Ok((&buf).into())
    }
}

// This function is kinda warcrimish, however I am content with its existence
pub async fn init_settings_storage() -> Result<()> {
    let mut buf = [0u8; core::mem::size_of::<Settings>()];
    STORAGE
        .get()
        .lock()
        .await
        .read(SETTINGS_STORAGE_OFFSET, &mut buf)
        .map_err(Error::StorageReadError)?;
    if (0..core::mem::size_of::<Settings>())
        .map(|idx| buf[idx])
        .all(|byte| byte == 0xff)
    {
        // This means that the settings storage was not initialized and all of the bytes in flash
        // memory are by default set to 0xff, thus we need to initialize this region with default
        // settings
        Settings::default().save().await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiSettings {
    ssid: heapless::String<32>,
    password: heapless::String<64>,
}

impl Default for WifiSettings {
    fn default() -> Self {
        Self {
            ssid: heapless::String::from_str(WIFI_SSID).unwrap(),
            password: heapless::String::from_str(WIFI_PASSWORD).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PresetSettings {
    #[serde(rename = "b")]
    pub brightness: u8,
    #[serde(rename = "sp")]
    pub speed: u8,
    #[serde(rename = "sc")]
    pub scale: u8,
}

impl Default for PresetSettings {
    fn default() -> Self {
        Self {
            brightness: 50,
            speed: 255,
            scale: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PresetId(u8);

impl PresetId {
    pub fn new_fallible(id: u8) -> Result<Self> {
        match id {
            0..PRESET_COUNT => Ok(Self(id)),
            _ => Err(Error::PresetIdOutOfBounds),
        }
    }

    pub fn id(&self) -> u8 {
        self.0
    }
}

impl core::fmt::Display for PresetId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts((p as *const T) as *const u8, core::mem::size_of::<T>()) }
}
