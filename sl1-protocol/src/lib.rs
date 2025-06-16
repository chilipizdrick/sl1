#![no_std]

pub type PresetId = u8;

pub const MESSAGE_BUFFER_LENGTH: usize = 1024;

#[derive(Debug)]
pub enum VersionError {
    InvalidProtocolVersionCode,
}

impl core::fmt::Display for VersionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Version {
    V1 = 0x01,
}

impl TryFrom<u8> for Version {
    type Error = self::VersionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Version::V1),
            _ => Err(VersionError::InvalidProtocolVersionCode),
        }
    }
}

#[derive(Debug)]
pub enum MethodError {
    InvalidProtocolMethodCode,
}

impl core::fmt::Display for MethodError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Method {
    Error = 0x00,

    GetPing = 0x01,
    GetIsOn = 0x02,
    GetCurrentPresetId = 0x03,
    GetPresetInfo = 0x04,
    GetSettings = 0x05,
    GetCurrentPresetSettings = 0x06,
    GetWifiSettings = 0x07,

    SetToggle = 0x08,
    SetTurnOn = 0x09,
    SetTurnOff = 0x0a,
    SetPreset = 0x0b,
    SetSettings = 0x0c,
    SetWifiSettings = 0x0d,
    SetCurrentPresetSettings = 0x0e,
    SetBrightness = 0x0f,
    SetSpeed = 0x10,
    SetScale = 0x11,
    SaveSettings = 0x12,
}

impl TryFrom<u8> for Method {
    type Error = self::MethodError;

    fn try_from(value: u8) -> Result<Self, self::MethodError> {
        match value {
            0x00 => Ok(Self::Error),
            0x01 => Ok(Self::GetPing),
            0x02 => Ok(Self::GetIsOn),
            0x03 => Ok(Self::GetCurrentPresetId),
            0x04 => Ok(Self::GetPresetInfo),
            0x05 => Ok(Self::GetSettings),
            0x06 => Ok(Self::GetCurrentPresetSettings),
            0x07 => Ok(Self::GetWifiSettings),
            0x08 => Ok(Self::SetToggle),
            0x09 => Ok(Self::SetTurnOn),
            0x0a => Ok(Self::SetTurnOff),
            0x0b => Ok(Self::SetPreset),
            0x0c => Ok(Self::SetSettings),
            0x0d => Ok(Self::SetWifiSettings),
            0x0e => Ok(Self::GetCurrentPresetSettings),
            0x0f => Ok(Self::SetBrightness),
            0x10 => Ok(Self::SetSpeed),
            0x11 => Ok(Self::SetScale),
            0x12 => Ok(Self::SaveSettings),
            _ => Err(MethodError::InvalidProtocolMethodCode),
        }
    }
}
