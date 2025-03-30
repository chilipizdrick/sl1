use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    time::Duration,
};

use crate::error::{Error, Result};

pub type PresetId = u8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    wifi_settings: DeviceWifiSettings,
    preset_settings: Vec<PresetSettings>,
    current_preset_id: PresetId,
    is_on: bool,
}

impl DeviceSettings {
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
    brightness: u8,
    speed: u8,
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
    pub fn new(id: PresetId, name: String) -> Self {
        Self { id, name }
    }

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

    pub fn connect(&self) -> Result<DeviceConnection> {
        log::info!("Connecting to device at {}", self.addr());
        DeviceConnection::new(self.addr())
    }
}

#[derive(Clone, Debug)]
enum ClientMessage {
    Get(GetClientMessage),
    Set(SetClientMessage),
}

impl ClientMessage {
    pub fn method_id(&self) -> u8 {
        match self {
            ClientMessage::Get(message) => message.method_id(),
            ClientMessage::Set(message) => message.method_id(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum GetClientMessage {
    Ping,
    CurrentPresetId,
    PresetInfo,
    Settings,
    WifiSettings,
    PresetSettings,
}

impl GetClientMessage {
    pub fn method_id(&self) -> u8 {
        match self {
            GetClientMessage::Ping => 0x01,
            GetClientMessage::CurrentPresetId => 0x02,
            GetClientMessage::PresetInfo => 0x03,
            GetClientMessage::Settings => 0x04,
            GetClientMessage::WifiSettings => 0x05,
            GetClientMessage::PresetSettings => 0x06,
        }
    }
}

#[derive(Clone, Debug)]
enum SetClientMessage {
    Toggle,
    TurnOn,
    TurnOff,
    SetPreset(PresetId),
    SetSettings(DeviceSettings),
    SetWifiSettings(DeviceWifiSettings),
    SetPresetSettings(PresetSettings),
    SetBrightness(u8),
    SetSpeed(u8),
    SetScale(u8),
}

impl SetClientMessage {
    pub fn method_id(&self) -> u8 {
        match self {
            SetClientMessage::Toggle => 0x07,
            SetClientMessage::TurnOn => 0x08,
            SetClientMessage::TurnOff => 0x09,
            SetClientMessage::SetPreset(_) => 0x0A,
            SetClientMessage::SetSettings(_) => 0x0B,
            SetClientMessage::SetWifiSettings(_) => 0x0C,
            SetClientMessage::SetPresetSettings(_) => 0x0D,
            SetClientMessage::SetBrightness(_) => 0x0E,
            SetClientMessage::SetSpeed(_) => 0x0F,
            SetClientMessage::SetScale(_) => 0x10,
        }
    }
}

// Should be at least 512 bytes
const BUFFER_SIZE: usize = 1024;

#[derive(Debug)]
pub struct DeviceConnection {
    stream: TcpStream,
    send_buffer: [u8; BUFFER_SIZE],
    recv_buffer: [u8; BUFFER_SIZE],
}

#[allow(unused)]
impl DeviceConnection {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        const SEND_RECV_TIMEOUT: Duration = Duration::from_millis(500);
        const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);

        let socket =
            TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(Error::DeviceConnection)?;
        socket
            .set_read_timeout(Some(SEND_RECV_TIMEOUT))
            .map_err(Error::DeviceConnection)?;
        socket
            .set_write_timeout(Some(SEND_RECV_TIMEOUT))
            .map_err(Error::DeviceConnection)?;

        Ok(Self {
            stream: socket,
            send_buffer: [0; BUFFER_SIZE],
            recv_buffer: [0; BUFFER_SIZE],
        })
    }

    pub fn set_blocking(&self) {
        self.stream.set_nonblocking(false);
    }

    pub fn set_nonblocking(&self) {
        self.stream.set_nonblocking(true);
    }

    pub fn ping(&mut self) -> Result<()> {
        self.send_message(ClientMessage::Get(GetClientMessage::Ping))?;
        self.recieve()?;
        Ok(())
    }

    pub fn get_current_preset_id(&mut self) -> Result<PresetId> {
        self.send_message(ClientMessage::Get(GetClientMessage::CurrentPresetId))?;
        let len = self.recieve()?;
        let message =
            String::from_utf8(self.recv_buffer[2..len].to_vec()).map_err(Error::FromUtf8)?;
        let res: u8 = serde_json::from_str(&message).map_err(|_| Error::Deserialization)?;
        Ok(res)
    }

    pub fn get_preset_info(&mut self) -> Result<Vec<Preset>> {
        self.send_message(ClientMessage::Get(GetClientMessage::PresetInfo))?;
        let len = self.recieve()?;
        let message =
            String::from_utf8(self.recv_buffer[2..len].to_vec()).map_err(Error::FromUtf8)?;
        let res: Vec<crate::device::Preset> =
            serde_json::from_str(&message).map_err(|_| Error::Deserialization)?;
        Ok(res)
    }

    pub fn get_device_settings(&mut self) -> Result<DeviceSettings> {
        self.send_message(ClientMessage::Get(GetClientMessage::Settings))?;
        let len = self.recieve()?;
        let message =
            String::from_utf8(self.recv_buffer[2..len].to_vec()).map_err(Error::FromUtf8)?;
        let res: crate::device::DeviceSettings =
            serde_json::from_str(&message).map_err(|_| Error::Deserialization)?;
        Ok(res)
    }

    pub fn get_device_wifi_settings(&mut self) -> Result<DeviceWifiSettings> {
        self.send_message(ClientMessage::Get(GetClientMessage::WifiSettings))?;
        let len = self.recieve()?;
        let message =
            String::from_utf8(self.recv_buffer[2..len].to_vec()).map_err(Error::FromUtf8)?;
        let res: crate::device::DeviceWifiSettings =
            serde_json::from_str(&message).map_err(|_| Error::Deserialization)?;
        Ok(res)
    }

    pub fn get_preset_settings(&mut self) -> Result<PresetSettings> {
        self.send_message(ClientMessage::Get(GetClientMessage::PresetSettings))?;
        let len = self.recieve()?;
        let message =
            String::from_utf8(self.recv_buffer[2..len].to_vec()).map_err(Error::FromUtf8)?;
        let res: crate::device::PresetSettings =
            serde_json::from_str(&message).map_err(|_| Error::Deserialization)?;
        Ok(res)
    }

    pub fn toggle(&mut self) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::Toggle))?;
        self.recieve()?;
        Ok(())
    }

    pub fn turn_on(&mut self) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::TurnOn))?;
        self.recieve()?;
        Ok(())
    }

    pub fn turn_off(&mut self) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::TurnOff))?;
        self.recieve()?;
        Ok(())
    }

    pub fn set_preset(&mut self, preset_id: PresetId) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetPreset(preset_id)))?;
        self.recieve()?;
        Ok(())
    }

    pub fn set_settings(&mut self, settings: crate::device::DeviceSettings) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetSettings(settings)))?;
        Ok(())
    }

    pub fn set_wifi_settings(
        &mut self,
        wifi_settings: crate::device::DeviceWifiSettings,
    ) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetWifiSettings(
            wifi_settings,
        )))?;
        Ok(())
    }

    pub fn set_preset_settings(
        &mut self,
        preset_settings: crate::device::PresetSettings,
    ) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetPresetSettings(
            preset_settings,
        )))?;
        self.recieve()?;
        Ok(())
    }

    pub fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetBrightness(
            brightness,
        )))?;
        self.recieve()?;
        Ok(())
    }

    pub fn set_speed(&mut self, speed: u8) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetSpeed(speed)))?;
        self.recieve()?;
        Ok(())
    }

    pub fn set_scale(&mut self, scale: u8) -> Result<()> {
        self.send_message(ClientMessage::Set(SetClientMessage::SetScale(scale)))?;
        self.recieve()?;
        Ok(())
    }

    fn send_message(&mut self, message: ClientMessage) -> Result<()> {
        self.send_buffer[0] = 0x01;
        match message {
            ClientMessage::Get(get_message) => self.send_get_message(get_message),
            ClientMessage::Set(set_message) => self.send_set_message(set_message),
        }
    }

    // On success returns the number of bytes received
    fn recieve(&mut self) -> Result<usize> {
        self.stream
            .read(&mut self.recv_buffer)
            .map_err(Error::DeviceRecieve)
    }

    fn send_get_message(&mut self, message: GetClientMessage) -> Result<()> {
        self.send_message_without_data(ClientMessage::Get(message))
    }

    fn send_set_message(&mut self, message: SetClientMessage) -> Result<()> {
        self.send_buffer[1] = message.method_id();
        match message {
            SetClientMessage::Toggle | SetClientMessage::TurnOff | SetClientMessage::TurnOn => {
                self.send(0)?;
            }
            SetClientMessage::SetPreset(preset_id) => {
                self.send_buffer[2] = preset_id;
                self.send(1)?;
            }
            SetClientMessage::SetSettings(settings) => {
                let data = serde_json::to_string(&settings).map_err(|_| Error::Serialization)?;
                let data = data.as_bytes();
                self.send_buffer[2..data.len() + 2].copy_from_slice(data);
                self.send(data.len())?;
            }
            SetClientMessage::SetWifiSettings(wifi_settings) => {
                let data =
                    serde_json::to_string(&wifi_settings).map_err(|_| Error::Serialization)?;
                let data = data.as_bytes();
                self.send_buffer[2..data.len() + 2].copy_from_slice(data);
                self.send(data.len())?;
            }
            SetClientMessage::SetPresetSettings(preset_settings) => {
                let data =
                    serde_json::to_string(&preset_settings).map_err(|_| Error::Serialization)?;
                let data = data.as_bytes();
                self.send_buffer[2..data.len() + 2].copy_from_slice(data);
                self.send(data.len())?;
            }
            SetClientMessage::SetBrightness(brightness) => {
                self.send_buffer[2] = brightness;
                self.send(1)?;
            }
            SetClientMessage::SetSpeed(speed) => {
                self.send_buffer[2] = speed;
                self.send(1)?;
            }
            SetClientMessage::SetScale(scale) => {
                self.send_buffer[2] = scale;
                self.send(1)?;
            }
        }
        Ok(())
    }

    fn send_message_without_data(&mut self, message: ClientMessage) -> Result<()> {
        self.send_buffer[1] = message.method_id();
        self.send(0)?;
        Ok(())
    }

    fn send(&mut self, len: usize) -> Result<()> {
        self.stream
            .write_all(&self.send_buffer[..len + 2])
            .map_err(Error::DeviceSend)
    }
}
