use alloc::string::{String, ToString};
use core::sync::atomic::Ordering;
use embassy_net::{
    udp::{PacketMetadata, UdpMetadata, UdpSocket},
    Runner, Stack,
};
use esp_hal::reset::software_reset;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};

use crate::{
    settings::{PresetId, PresetSettings, Settings, WifiSettings},
    Error, Result, MESSAGE_BUFFER_LENGTH, MINIMAL_CLIENT_MESSAGE_LENGTH, PRESET_INFO, SERVER_PORT,
    SETTINGS, SHOULD_UPDATE,
};

#[embassy_executor::task]
pub async fn net_task(mut stack_runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) -> ! {
    stack_runner.run().await
}

#[derive(Clone, Copy, Debug)]
enum ProtocolVersion {
    V1,
}

impl TryFrom<u8> for ProtocolVersion {
    type Error = crate::Error;
    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            1 => Ok(ProtocolVersion::V1),
            _ => Err(Error::InvalidProtocolVersion),
        }
    }
}

impl ProtocolVersion {
    fn as_u8(&self) -> u8 {
        match self {
            ProtocolVersion::V1 => 1,
        }
    }
}

#[derive(Clone, Debug)]
enum ClientMessage {
    Get(GetClientMessage),
    Set(SetClientMessage),
}

#[derive(Clone, Copy, Debug)]
enum GetClientMessage {
    Ping,
    IsOn,
    CurrentPresetId,
    PresetInfo,
    Settings,
    WifiSettings,
    CurrentPresetSettings,
}

#[derive(Clone, Debug)]
enum SetClientMessage {
    Toggle,
    TurnOn,
    TurnOff,
    Preset(PresetId),
    Settings(Settings),
    WifiSettings(WifiSettings),
    CurrentPresetSettings(PresetSettings),
    Brightness(u8),
    Speed(u8),
    Scale(u8),
}

#[allow(unreachable_patterns)]
impl ClientMessage {
    fn from_message(buf: &[u8]) -> Result<Self> {
        let version = ProtocolVersion::try_from(buf[0])?;
        match version {
            ProtocolVersion::V1 => {}
            _ => return Err(Error::UnsupportedProtocolVersion),
        };

        let method = buf[1];

        match method {
            0x01 => Ok(ClientMessage::Get(GetClientMessage::Ping)),
            0x02 => Ok(ClientMessage::Get(GetClientMessage::IsOn)),
            0x03 => Ok(ClientMessage::Get(GetClientMessage::CurrentPresetId)),
            0x04 => Ok(ClientMessage::Get(GetClientMessage::PresetInfo)),
            0x05 => Ok(ClientMessage::Get(GetClientMessage::Settings)),
            0x06 => Ok(ClientMessage::Get(GetClientMessage::WifiSettings)),
            0x07 => Ok(ClientMessage::Get(GetClientMessage::CurrentPresetSettings)),

            0x08 => Ok(ClientMessage::Set(SetClientMessage::Toggle)),
            0x09 => Ok(ClientMessage::Set(SetClientMessage::TurnOn)),
            0x0a => Ok(ClientMessage::Set(SetClientMessage::TurnOff)),
            0x0B => {
                let preset_id = PresetId::new_fallible(buf[2])?;
                Ok(ClientMessage::Set(SetClientMessage::Preset(preset_id)))
            }
            0x0C => {
                let settings: Settings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::DeserializationError)?;
                Ok(ClientMessage::Set(SetClientMessage::Settings(settings)))
            }
            0x0D => {
                let wifi_settings: WifiSettings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::DeserializationError)?;
                Ok(ClientMessage::Set(SetClientMessage::WifiSettings(
                    wifi_settings,
                )))
            }
            0x0E => {
                let preset_settings: PresetSettings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::DeserializationError)?;
                Ok(ClientMessage::Set(SetClientMessage::CurrentPresetSettings(
                    preset_settings,
                )))
            }
            0x0F => {
                let brightness = buf[2];
                Ok(ClientMessage::Set(SetClientMessage::Brightness(brightness)))
            }
            0x10 => {
                let speed = buf[2];
                Ok(ClientMessage::Set(SetClientMessage::Speed(speed)))
            }
            0x11 => {
                let scale = buf[2];
                Ok(ClientMessage::Set(SetClientMessage::Scale(scale)))
            }

            _ => Err(Error::UnsupportedClientMessageMethod),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ServerMessage {
    Error,

    GetPing,
    GetIsOn,
    GetCurrentPresetId,
    GetPresetInfo,
    GetSettings,
    GetCurrentPresetSettings,
    GetWifiSettings,

    SetToggle,
    SetTurnOn,
    SetTurnOff,
    SetPreset,
    SetSettings,
    SetWifiSettings,
    SetCurrentPresetSettings,
    SetBrightness,
    SetSpeed,
    SetScale,
}

impl ServerMessage {
    fn method_id(&self) -> u8 {
        match self {
            ServerMessage::Error => 0x00,
            ServerMessage::GetPing => 0x01,
            ServerMessage::GetIsOn => 0x02,
            ServerMessage::GetCurrentPresetId => 0x03,
            ServerMessage::GetPresetInfo => 0x04,
            ServerMessage::GetSettings => 0x05,
            ServerMessage::GetCurrentPresetSettings => 0x06,
            ServerMessage::GetWifiSettings => 0x07,
            ServerMessage::SetToggle => 0x08,
            ServerMessage::SetTurnOn => 0x09,
            ServerMessage::SetTurnOff => 0x0A,
            ServerMessage::SetPreset => 0x0B,
            ServerMessage::SetSettings => 0x0C,
            ServerMessage::SetWifiSettings => 0x0D,
            ServerMessage::SetCurrentPresetSettings => 0x0E,
            ServerMessage::SetBrightness => 0x0F,
            ServerMessage::SetSpeed => 0x10,
            ServerMessage::SetScale => 0x11,
        }
    }

    fn from_set_client_message(message: &SetClientMessage) -> Self {
        match message {
            SetClientMessage::Toggle => ServerMessage::SetToggle,
            SetClientMessage::TurnOn => ServerMessage::SetTurnOn,
            SetClientMessage::TurnOff => ServerMessage::SetTurnOff,
            SetClientMessage::Preset(_) => ServerMessage::SetPreset,
            SetClientMessage::Settings(_) => ServerMessage::SetSettings,
            SetClientMessage::WifiSettings(_) => ServerMessage::SetWifiSettings,
            SetClientMessage::CurrentPresetSettings(_) => ServerMessage::SetCurrentPresetSettings,
            SetClientMessage::Brightness(_) => ServerMessage::SetBrightness,
            SetClientMessage::Speed(_) => ServerMessage::SetSpeed,
            SetClientMessage::Scale(_) => ServerMessage::SetScale,
        }
    }

    fn from_get_client_message(message: &GetClientMessage) -> Self {
        match message {
            GetClientMessage::Ping => ServerMessage::GetPing,
            GetClientMessage::IsOn => ServerMessage::GetIsOn,
            GetClientMessage::CurrentPresetId => ServerMessage::GetCurrentPresetId,
            GetClientMessage::PresetInfo => ServerMessage::GetPresetInfo,
            GetClientMessage::Settings => ServerMessage::GetSettings,
            GetClientMessage::WifiSettings => ServerMessage::GetWifiSettings,
            GetClientMessage::CurrentPresetSettings => ServerMessage::GetCurrentPresetSettings,
        }
    }

    async fn from_client_message_fallible(message: ClientMessage) -> Result<Self> {
        match message {
            ClientMessage::Get(message) => Ok(Self::from_get_client_message(&message)),
            ClientMessage::Set(message) => {
                let mut settings = SETTINGS.get().lock().await;
                SHOULD_UPDATE.store(true, Ordering::Relaxed);
                let response_message = Self::from_set_client_message(&message);
                match message {
                    SetClientMessage::Toggle => {
                        settings.is_on = !settings.is_on;
                        settings.save().await?;
                    }
                    SetClientMessage::TurnOn => {
                        settings.is_on = true;
                        settings.save().await?;
                    }
                    SetClientMessage::TurnOff => {
                        settings.is_on = false;
                        settings.save().await?;
                    }
                    SetClientMessage::Preset(preset_id) => {
                        settings.current_preset_id = preset_id;
                        settings.save().await?;
                    }
                    SetClientMessage::Settings(new_settings) => {
                        *settings = new_settings;
                        settings.save().await?;
                        software_reset();
                    }
                    SetClientMessage::WifiSettings(wifi_settings) => {
                        settings.wifi_settings = wifi_settings;
                        settings.save().await?;
                        software_reset();
                    }
                    SetClientMessage::CurrentPresetSettings(preset_settings) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize] = preset_settings;
                        settings.save().await?;
                    }
                    SetClientMessage::Brightness(brightness) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize].brightness =
                            brightness;
                        settings.save().await?;
                    }
                    SetClientMessage::Speed(speed) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize].speed = speed;
                        settings.save().await?;
                    }
                    SetClientMessage::Scale(scale) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize].scale = scale;
                        settings.save().await?;
                    }
                };
                Ok(response_message)
            }
        }
    }

    async fn from_client_message(message: ClientMessage) -> Self {
        match Self::from_client_message_fallible(message).await {
            Ok(message) => message,
            Err(e) => {
                log::error!("Error processing client message: {:?}", e);
                ServerMessage::Error
            }
        }
    }

    async fn send(
        &self,
        socket: &mut UdpSocket<'_>,
        buf: &mut [u8],
        addr: UdpMetadata,
    ) -> Result<()> {
        buf[0] = ProtocolVersion::V1.as_u8();
        buf[1] = self.method_id();
        let mut payload: String = String::new();
        let settings = SETTINGS.get().lock().await;

        match self {
            ServerMessage::GetIsOn => {
                buf[2] = settings.is_on as u8;
            }
            ServerMessage::GetCurrentPresetId => {
                payload = settings.current_preset_id.id().to_string()
            }
            ServerMessage::GetPresetInfo => payload = String::from(PRESET_INFO),
            ServerMessage::GetSettings => {
                payload = serde_json::to_string(&*settings).map_err(Error::SerializationError)?;
            }
            ServerMessage::GetCurrentPresetSettings => {
                let current_preset_id = settings.current_preset_id.id();
                payload =
                    serde_json::to_string(&settings.preset_settings[current_preset_id as usize])
                        .map_err(Error::SerializationError)?;
            }
            ServerMessage::GetWifiSettings => {
                payload = serde_json::to_string(&settings.wifi_settings)
                    .map_err(Error::SerializationError)?;
            }
            _ => {}
        }

        let payload_bytes = payload.as_bytes();
        buf[2..2 + payload_bytes.len()].copy_from_slice(payload_bytes);
        let message_len = 2 + payload_bytes.len();
        socket
            .send_to(&buf[..message_len], addr)
            .await
            .map_err(Error::SendError)
    }
}

#[embassy_executor::task]
pub async fn server_task(stack: Stack<'static>) -> ! {
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; MESSAGE_BUFFER_LENGTH];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; MESSAGE_BUFFER_LENGTH];
    let mut message_buffer = [0; MESSAGE_BUFFER_LENGTH];

    stack.wait_config_up().await;
    match stack.config_v4() {
        Some(config) => log::info!("Aquired IP address: {}", config.address),
        None => log::warn!("Failed to aquire IP address"),
    }

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(SERVER_PORT).unwrap();
    log::info!("Server ready!");

    loop {
        let (rx_size, from_addr) = match socket.recv_from(&mut message_buffer).await {
            Ok((size, addr)) => (size, addr),
            Err(e) => {
                log::error!("Error recieving data from UDP connection: {:?}", e);
                continue;
            }
        };

        if rx_size < MINIMAL_CLIENT_MESSAGE_LENGTH {
            log::warn!(
                "Received message, of length {} - smaller than minimal accepted - {}",
                rx_size,
                MINIMAL_CLIENT_MESSAGE_LENGTH
            );
            continue;
        }

        let request = match ClientMessage::from_message(&message_buffer[..rx_size]) {
            Ok(message) => message,
            Err(e) => {
                log::error!("Error parsing recieved message: {:?}", e);
                continue;
            }
        };

        log::debug!("Request: {:?}", &request);

        let response = ServerMessage::from_client_message(request).await;

        response
            .send(&mut socket, &mut message_buffer, from_addr)
            .await
            .unwrap_or_else(|e| {
                log::error!("Error sending response to client: {:?}", e);
            });
    }
}
