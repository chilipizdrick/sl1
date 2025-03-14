use alloc::string::String;
use core::sync::atomic::Ordering;
use embassy_net::{
    udp::{PacketMetadata, UdpMetadata, UdpSocket},
    Runner, Stack,
};
use esp_hal::reset::software_reset;
use esp_println::dbg;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};

use crate::{
    settings::{PresetId, PresetSettings, Settings, WifiSettings},
    Error, Result, MINIMAL_CLIENT_MESSAGE_LENGTH, PONG, PRESET_INFO, SERVER_PORT, SETTINGS,
    SHOULD_UPDATE, UDP_MESSAGE_BUFFER_LENGTH,
};

#[embassy_executor::task]
pub async fn net_task(mut stack_runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) -> ! {
    stack_runner.run().await
}

#[derive(Clone, Copy, Debug)]
enum ProtocolVersion {
    V0,
}

impl TryFrom<u8> for ProtocolVersion {
    type Error = crate::Error;
    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            0 => Ok(ProtocolVersion::V0),
            _ => Err(Error::InvalidProtocolVersion),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum GetClientMessage {
    PresetInfo,
    Settings,
    WifiSettings,
    PresetSettings,
}

#[derive(Clone, Debug)]
enum SetClientMessage {
    Toggle,
    TurnOn,
    TurnOff,
    SetPreset(PresetId),
    SetSettings(Settings),
    SetWifiSettings(WifiSettings),
    SetPresetSettings(PresetSettings),
    SetBrightness(u8),
    SetSpeed(u8),
    SetScale(u8),
}

#[derive(Clone, Debug)]
enum ClientMessage {
    Ping,
    Get(GetClientMessage),
    Set(SetClientMessage),
}

#[allow(unreachable_patterns)]
impl TryFrom<&[u8]> for ClientMessage {
    type Error = crate::Error;
    fn try_from(buf: &[u8]) -> Result<Self> {
        let version = ProtocolVersion::try_from(buf[0])?;
        match version {
            ProtocolVersion::V0 => {}
            _ => return Err(Error::UnsupportedProtocolVersion),
        };

        let method = buf[1];

        match method {
            0 => Ok(ClientMessage::Ping),

            1 => Ok(ClientMessage::Get(GetClientMessage::PresetInfo)),
            2 => Ok(ClientMessage::Get(GetClientMessage::Settings)),
            3 => Ok(ClientMessage::Get(GetClientMessage::WifiSettings)),
            4 => Ok(ClientMessage::Get(GetClientMessage::PresetSettings)),

            5 => Ok(ClientMessage::Set(SetClientMessage::Toggle)),
            6 => Ok(ClientMessage::Set(SetClientMessage::TurnOn)),
            7 => Ok(ClientMessage::Set(SetClientMessage::TurnOff)),
            8 => {
                let preset_id = PresetId::new_fallible(buf[2] as usize)?;
                Ok(ClientMessage::Set(SetClientMessage::SetPreset(preset_id)))
            }
            9 => {
                let settings: Settings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::DeserializationError)?;
                Ok(ClientMessage::Set(SetClientMessage::SetSettings(settings)))
            }
            10 => {
                let wifi_settings: WifiSettings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::DeserializationError)?;
                Ok(ClientMessage::Set(SetClientMessage::SetWifiSettings(
                    wifi_settings,
                )))
            }
            11 => {
                let preset_settings: PresetSettings =
                    serde_json::from_slice(&buf[2..]).map_err(Error::DeserializationError)?;
                Ok(ClientMessage::Set(SetClientMessage::SetPresetSettings(
                    preset_settings,
                )))
            }
            12 => {
                let brightness = buf[2];
                Ok(ClientMessage::Set(SetClientMessage::SetBrightness(
                    brightness,
                )))
            }
            13 => {
                let speed = buf[2];
                Ok(ClientMessage::Set(SetClientMessage::SetSpeed(speed)))
            }
            14 => {
                let scale = buf[2];
                Ok(ClientMessage::Set(SetClientMessage::SetScale(scale)))
            }

            _ => Err(Error::UnsupportedClientMessageMethod),
        }
    }
}

// impl ClientMessage {
//     fn method_id(&self) -> u8 {
//         match self {
//             ClientMessage::Ping => 0,
//             ClientMessage::GetClientMessage(msg) => match msg {
//                 GetClientMessage::GetPresetInfo => 1,
//                 GetClientMessage::GetSettings => 2,
//                 GetClientMessage::GetWifiSettings => 3,
//                 GetClientMessage::GetPresetSettings => 4,
//             },
//             ClientMessage::SetClientMessage(msg) => match msg {
//                 SetClientMessage::Toggle => 5,
//                 SetClientMessage::TurnOn => 6,
//                 SetClientMessage::TurnOff => 7,
//                 SetClientMessage::SetPreset(_) => 8,
//                 SetClientMessage::SetSettings(_) => 9,
//                 SetClientMessage::SetWifiSettings(_) => 10,
//                 SetClientMessage::SetPresetSettings(_) => 11,
//                 SetClientMessage::SetBrightness(_) => 12,
//                 SetClientMessage::SetSpeed(_) => 13,
//                 SetClientMessage::SetScale(_) => 14,
//             },
//         }
//     }
// }

#[derive(Clone, Copy, Debug)]
enum SendServerMessage {
    PresetInfo,
    Settings,
    PresetSettings,
    WifiSettings,
}

#[derive(Clone, Copy, Debug)]
enum OkServerMessage {
    Pong,
    SendServerMessage(SendServerMessage),
    None,
}

#[allow(unused)]
#[derive(Clone, Copy, Debug)]
enum ServerMessage {
    Err,
    Ok(OkServerMessage),
}

impl ServerMessage {
    fn method_id(&self) -> u8 {
        match self {
            ServerMessage::Err => 0,
            ServerMessage::Ok(msg) => match msg {
                OkServerMessage::None => 1,
                OkServerMessage::Pong => 2,
                OkServerMessage::SendServerMessage(msg) => match msg {
                    SendServerMessage::PresetInfo => 3,
                    SendServerMessage::Settings => 4,
                    SendServerMessage::PresetSettings => 5,
                    SendServerMessage::WifiSettings => 6,
                },
            },
        }
    }

    async fn from_client_message(msg: ClientMessage) -> Result<Self> {
        match msg {
            ClientMessage::Ping => Ok(ServerMessage::Ok(OkServerMessage::Pong)),

            ClientMessage::Get(msg) => match msg {
                GetClientMessage::PresetInfo => Ok(ServerMessage::Ok(
                    OkServerMessage::SendServerMessage(SendServerMessage::PresetInfo),
                )),
                GetClientMessage::Settings => Ok(ServerMessage::Ok(
                    OkServerMessage::SendServerMessage(SendServerMessage::Settings),
                )),
                GetClientMessage::WifiSettings => Ok(ServerMessage::Ok(
                    OkServerMessage::SendServerMessage(SendServerMessage::WifiSettings),
                )),
                GetClientMessage::PresetSettings => Ok(ServerMessage::Ok(
                    OkServerMessage::SendServerMessage(SendServerMessage::PresetSettings),
                )),
            },

            ClientMessage::Set(msg) => {
                let mut settings = SETTINGS.get().lock().await;
                match msg {
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
                    SetClientMessage::SetPreset(preset_id) => {
                        settings.current_preset_id = preset_id;
                        settings.save().await?;
                    }
                    SetClientMessage::SetSettings(new_settings) => {
                        *settings = new_settings;
                        settings.save().await?;
                        software_reset();
                    }
                    SetClientMessage::SetWifiSettings(wifi_settings) => {
                        settings.wifi_settings = wifi_settings;
                        settings.save().await?;
                        software_reset();
                    }
                    SetClientMessage::SetPresetSettings(preset_settings) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id] = preset_settings;
                        settings.save().await?;
                    }
                    SetClientMessage::SetBrightness(brightness) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id].brightness = brightness;
                        settings.save().await?;
                    }
                    SetClientMessage::SetSpeed(speed) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id].speed = speed;
                        settings.save().await?;
                    }
                    SetClientMessage::SetScale(scale) => {
                        let current_preset_id = settings.current_preset_id.id();
                        settings.preset_settings[current_preset_id].scale = scale;
                        settings.save().await?;
                    }
                };
                SHOULD_UPDATE.store(true, Ordering::Relaxed);
                Ok(ServerMessage::Ok(OkServerMessage::None))
            }
        }
    }

    async fn send(
        &self,
        socket: &mut UdpSocket<'_>,
        buf: &mut [u8],
        addr: UdpMetadata,
    ) -> Result<()> {
        buf[0] = ProtocolVersion::V0 as u8;
        buf[1] = self.method_id();
        let mut payload: String = String::new();
        let settings = SETTINGS.get().lock().await;

        match self {
            ServerMessage::Err => {}
            ServerMessage::Ok(msg) => match msg {
                OkServerMessage::None => {}
                OkServerMessage::Pong => {
                    payload = String::from(PONG);
                }
                OkServerMessage::SendServerMessage(msg) => match msg {
                    SendServerMessage::PresetInfo => payload = String::from(PRESET_INFO),
                    SendServerMessage::Settings => {
                        payload =
                            serde_json::to_string(&*settings).map_err(Error::SerializationError)?;
                    }
                    SendServerMessage::PresetSettings => {
                        let current_preset_id = settings.current_preset_id.id();
                        payload =
                            serde_json::to_string(&settings.preset_settings[current_preset_id])
                                .map_err(Error::SerializationError)?;
                    }
                    SendServerMessage::WifiSettings => {
                        payload = serde_json::to_string(&settings.wifi_settings)
                            .map_err(Error::SerializationError)?;
                    }
                },
            },
        }

        let payload_bytes = payload.as_bytes();
        buf[2..2 + payload_bytes.len()].copy_from_slice(payload_bytes);
        let msg_len = 2 + payload_bytes.len();
        socket
            .send_to(&buf[..msg_len], addr)
            .await
            .map_err(Error::SendError)?;
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn server_task(stack: Stack<'static>) -> ! {
    let mut udp_rx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_rx_buffer = [0; 1024];
    let mut udp_tx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_tx_buffer = [0; 1024];
    let mut msg_buffer = [0; UDP_MESSAGE_BUFFER_LENGTH];

    stack.wait_config_up().await;
    match stack.config_v4() {
        Some(config) => log::info!("Aquired IP address: {}", config.address),
        None => log::warn!("Failed to aquire IP address"),
    }

    let mut socket = UdpSocket::new(
        stack,
        &mut udp_rx_meta,
        &mut udp_rx_buffer,
        &mut udp_tx_meta,
        &mut udp_tx_buffer,
    );
    socket.bind(SERVER_PORT).unwrap();

    loop {
        let (rx_size, from_addr) = match socket.recv_from(&mut msg_buffer).await {
            Ok((size, addr)) => (size, addr),
            Err(e) => {
                log::error!("Error recieving data from UDP connection: {:?}", e);
                continue;
            }
        };

        if rx_size < MINIMAL_CLIENT_MESSAGE_LENGTH {
            log::warn!(
                "Received message from {}, of length {} - smaller than minimal accepted - {}",
                from_addr,
                rx_size,
                MINIMAL_CLIENT_MESSAGE_LENGTH
            );
            continue;
        }

        let request = match ClientMessage::try_from(&msg_buffer[..rx_size]) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Error parsing recieved message: {:?}", e);
                continue;
            }
        };

        dbg!(&request);

        let response = match ServerMessage::from_client_message(request).await {
            Ok(response) => response,
            Err(e) => {
                log::error!("Error processing recieved message: {:?}", e);
                continue;
            }
        };

        response
            .send(&mut socket, &mut msg_buffer, from_addr)
            .await
            .unwrap_or_else(|e| {
                log::error!("Error sending response to client: {:?}", e);
            });
    }
}
