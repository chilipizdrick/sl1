use alloc::string::{String, ToString};
use core::sync::atomic::Ordering;

use embassy_net::udp::{PacketMetadata, UdpMetadata, UdpSocket};
use embassy_net::{Runner, Stack};
use esp_hal::reset::software_reset;
use esp_hal::riscv::register::mstatus::set_tvm;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};

use sl1_protocol::{Method, Version};

use crate::settings::{PresetId, PresetSettings, Settings, WifiSettings};
use crate::{
    Error, MESSAGE_BUFFER_LENGTH, MINIMAL_CLIENT_MESSAGE_LENGTH, PRESET_INFO, Result, SERVER_PORT,
    SETTINGS, SHOULD_UPDATE,
};

#[embassy_executor::task]
pub async fn net_task(mut stack_runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) -> ! {
    stack_runner.run().await
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
    SaveSettings,
}

#[allow(unreachable_patterns)]
impl ClientMessage {
    fn from_message(buf: &[u8]) -> Result<Self> {
        use ClientMessage as CM;
        use GetClientMessage as GCM;
        use SetClientMessage as SCM;

        let version = Version::try_from(buf[0])?;
        match version {
            Version::V1 => {}
            _ => return Err(Error::UnsupportedProtocolVersion),
        };

        let method = buf[1];

        match method {
            0x01 => Ok(CM::Get(GCM::Ping)),
            0x02 => Ok(CM::Get(GCM::IsOn)),
            0x03 => Ok(CM::Get(GCM::CurrentPresetId)),
            0x04 => Ok(CM::Get(GCM::PresetInfo)),
            0x05 => Ok(CM::Get(GCM::Settings)),
            0x06 => Ok(CM::Get(GCM::WifiSettings)),
            0x07 => Ok(CM::Get(GCM::CurrentPresetSettings)),

            0x08 => Ok(CM::Set(SCM::Toggle)),
            0x09 => Ok(CM::Set(SCM::TurnOn)),
            0x0a => Ok(CM::Set(SCM::TurnOff)),
            0x0B => {
                let preset_id = PresetId::new_fallible(buf[2])?;
                Ok(CM::Set(SCM::Preset(preset_id)))
            }
            0x0C => {
                let settings: Settings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::Deserialization)?;
                Ok(CM::Set(SCM::Settings(settings)))
            }
            0x0D => {
                let wifi_settings: WifiSettings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::Deserialization)?;
                Ok(CM::Set(SCM::WifiSettings(wifi_settings)))
            }
            0x0E => {
                let preset_settings: PresetSettings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::Deserialization)?;
                Ok(CM::Set(SCM::CurrentPresetSettings(preset_settings)))
            }
            0x0F => {
                let brightness = buf[2];
                Ok(CM::Set(SCM::Brightness(brightness)))
            }
            0x10 => {
                let speed = buf[2];
                Ok(CM::Set(SCM::Speed(speed)))
            }
            0x11 => {
                let scale = buf[2];
                Ok(CM::Set(SCM::Scale(scale)))
            }
            0x12 => Ok(CM::Set(SCM::SaveSettings)),

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
    SaveSettings,
}

impl ServerMessage {
    fn method_id(&self) -> u8 {
        use ServerMessage as SM;

        match self {
            SM::Error => 0x00,
            SM::GetPing => 0x01,
            SM::GetIsOn => 0x02,
            SM::GetCurrentPresetId => 0x03,
            SM::GetPresetInfo => 0x04,
            SM::GetSettings => 0x05,
            SM::GetCurrentPresetSettings => 0x06,
            SM::GetWifiSettings => 0x07,
            SM::SetToggle => 0x08,
            SM::SetTurnOn => 0x09,
            SM::SetTurnOff => 0x0A,
            SM::SetPreset => 0x0B,
            SM::SetSettings => 0x0C,
            SM::SetWifiSettings => 0x0D,
            SM::SetCurrentPresetSettings => 0x0E,
            SM::SetBrightness => 0x0F,
            SM::SetSpeed => 0x10,
            SM::SetScale => 0x11,
            SM::SaveSettings => 0x12,
        }
    }

    fn from_set_client_message(message: &SetClientMessage) -> Self {
        use ServerMessage as SM;
        use SetClientMessage as SCM;

        match message {
            SCM::Toggle => SM::SetToggle,
            SCM::TurnOn => SM::SetTurnOn,
            SCM::TurnOff => SM::SetTurnOff,
            SCM::Preset(_) => SM::SetPreset,
            SCM::Settings(_) => SM::SetSettings,
            SCM::WifiSettings(_) => SM::SetWifiSettings,
            SCM::CurrentPresetSettings(_) => SM::SetCurrentPresetSettings,
            SCM::Brightness(_) => SM::SetBrightness,
            SCM::Speed(_) => SM::SetSpeed,
            SCM::Scale(_) => SM::SetScale,
            SCM::SaveSettings => SM::SaveSettings,
        }
    }

    fn from_get_client_message(message: &GetClientMessage) -> Self {
        use GetClientMessage as GCM;
        use ServerMessage as SM;

        match message {
            GCM::Ping => SM::GetPing,
            GCM::IsOn => SM::GetIsOn,
            GCM::CurrentPresetId => SM::GetCurrentPresetId,
            GCM::PresetInfo => SM::GetPresetInfo,
            GCM::Settings => SM::GetSettings,
            GCM::WifiSettings => SM::GetWifiSettings,
            GCM::CurrentPresetSettings => SM::GetCurrentPresetSettings,
        }
    }

    async fn from_client_message_fallible(message: ClientMessage) -> Result<Self> {
        match message {
            ClientMessage::Get(message) => Ok(Self::from_get_client_message(&message)),
            ClientMessage::Set(message) => {
                use SetClientMessage as SCM;

                let mut settings = SETTINGS.get().lock().await;

                let response_message = Self::from_set_client_message(&message);

                match message {
                    SCM::Toggle => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        settings.is_on = !settings.is_on;
                    }
                    SCM::TurnOn => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        settings.is_on = true;
                    }
                    SCM::TurnOff => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        settings.is_on = false;
                    }
                    SCM::Preset(preset_id) => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        settings.current_preset_id = preset_id;
                    }
                    SCM::Settings(new_settings) => {
                        *settings = new_settings;
                        settings.save().await?;
                        software_reset();
                    }
                    SCM::WifiSettings(wifi_settings) => {
                        settings.wifi_settings = wifi_settings;
                        settings.save().await?;
                        software_reset();
                    }
                    SCM::CurrentPresetSettings(preset_settings) => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize] = preset_settings;
                    }
                    SCM::Brightness(brightness) => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize].brightness =
                            brightness;
                    }
                    SCM::Speed(speed) => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize].speed = speed;
                    }
                    SCM::Scale(scale) => {
                        SHOULD_UPDATE.store(true, Ordering::Relaxed);
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id as usize].scale = scale;
                    }
                    SCM::SaveSettings => {
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
        use ServerMessage as SM;

        buf[0] = Version::V1 as u8;
        buf[1] = self.method_id();
        let mut payload: String = String::new();
        let settings = SETTINGS.get().lock().await;

        match self {
            SM::GetIsOn => {
                buf[2] = settings.is_on as u8;
            }
            SM::GetCurrentPresetId => payload = settings.current_preset_id.id().to_string(),
            SM::GetPresetInfo => payload = String::from(PRESET_INFO),
            SM::GetSettings => {
                payload = serde_json::to_string(&*settings).map_err(Error::Serialization)?;
            }
            SM::GetCurrentPresetSettings => {
                let current_preset_id = settings.current_preset_id.id();
                payload =
                    serde_json::to_string(&settings.preset_settings[current_preset_id as usize])
                        .map_err(Error::Serialization)?;
            }
            SM::GetWifiSettings => {
                payload =
                    serde_json::to_string(&settings.wifi_settings).map_err(Error::Serialization)?;
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
    let mut rx_buf = [0; MESSAGE_BUFFER_LENGTH];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buf = [0; MESSAGE_BUFFER_LENGTH];
    let mut message_buf = [0; MESSAGE_BUFFER_LENGTH];

    stack.wait_config_up().await;
    match stack.config_v4() {
        Some(config) => log::info!("Aquired IP address: {}", config.address),
        None => log::warn!("Failed to aquire IP address"),
    }

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    socket.bind(SERVER_PORT).unwrap();
    log::info!("Server ready!");

    loop {
        let (rx_size, from_addr) = match socket.recv_from(&mut message_buf).await {
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

        let request = match ClientMessage::from_message(&message_buf[..rx_size]) {
            Ok(message) => message,
            Err(e) => {
                log::error!("Error parsing recieved message: {:?}", e);
                continue;
            }
        };

        let response = ServerMessage::from_client_message(request).await;

        response
            .send(&mut socket, &mut message_buf, from_addr)
            .await
            .unwrap_or_else(|e| {
                log::error!("Error sending response to client: {:?}", e);
            });
    }
}
