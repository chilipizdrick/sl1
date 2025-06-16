use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use iced::futures::channel::mpsc;
use iced::futures::sink::SinkExt;
use iced::futures::{Stream, StreamExt};
use iced::stream;
use tokio::net::UdpSocket;

use crate::device::{DeviceSettings, DeviceWifiSettings, Preset, PresetId, PresetSettings};
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub enum Response {
    Ready(mpsc::Sender<Request>),
    Device(DeviceResponse),
}

#[derive(Debug, Clone)]
pub enum Request {
    SetDeviceAddr(SocketAddr),
    Get(GetRequest),
    Set(SetRequest),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum GetRequest {
    Ping,
    IsOn,
    CurrentPresetId,
    PresetInfo,
    Settings,
    WifiSettings,
    CurrentPresetSettings,
}

impl GetRequest {
    fn to_u8(&self) -> u8 {
        use GetRequest as GR;
        match self {
            GR::Ping => 0x01,
            GR::IsOn => 0x02,
            GR::CurrentPresetId => 0x03,
            GR::PresetInfo => 0x04,
            GR::Settings => 0x05,
            GR::WifiSettings => 0x06,
            GR::CurrentPresetSettings => 0x07,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum SetRequest {
    Toggle,
    TurnOn,
    TurnOff,
    Preset(PresetId),
    Settings(DeviceSettings),
    WifiSettings(DeviceWifiSettings),
    CurrentPresetSettings(PresetSettings),
    Brightness(u8),
    Speed(u8),
    Scale(u8),
    SaveSettings,
}

impl SetRequest {
    fn to_u8(&self) -> u8 {
        use SetRequest as SR;
        match self {
            SR::Toggle => 0x08,
            SR::TurnOn => 0x09,
            SR::TurnOff => 0x0A,
            SR::Preset(_) => 0x0B,
            SR::Settings(_) => 0x0C,
            SR::WifiSettings(_) => 0x0D,
            SR::CurrentPresetSettings(_) => 0x0E,
            SR::Brightness(_) => 0x0F,
            SR::Speed(_) => 0x10,
            SR::Scale(_) => 0x11,
            SR::SaveSettings => 0x12,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeviceResponse {
    Error,
    Get(DeviceGetResponse),
    Set(DeviceSetResponse),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum DeviceGetResponse {
    Ping,
    IsOn(bool),
    CurrentPresetId(PresetId),
    PresetInfo(Vec<Preset>),
    Settings(DeviceSettings),
    CurrentPresetSettings(PresetSettings),
    WifiSettings(DeviceWifiSettings),
}

#[derive(Debug, Clone)]
pub enum DeviceSetResponse {
    Toggle,
    TurnOn,
    TurnOff,
    Preset,
    Settings,
    WifiSettings,
    CurrentPresetSettings,
    Brightness,
    Speed,
    Scale,
    SaveSettings,
}

struct Sender {
    socket: Arc<UdpSocket>,
    device_addr: Option<SocketAddr>,
    send_buff: [u8; 1024],
}

impl Sender {
    fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket,
            device_addr: None,
            send_buff: [0; 1024],
        }
    }

    async fn handle_request(&mut self, request: Request) -> Result<()> {
        match request {
            Request::SetDeviceAddr(addr) => {
                self.set_device_addr(addr).await;
                Ok(())
            }
            Request::Get(request) => self.send_get_request(request).await,
            Request::Set(request) => self.send_set_request(request).await,
        }
    }

    async fn send_get_request(&mut self, request: GetRequest) -> Result<()> {
        self.send_buff[0] = 0x01;
        self.send_buff[1] = request.to_u8();
        self.send_with_timeout(2).await?;
        Ok(())
    }

    async fn send_set_request(&mut self, request: SetRequest) -> Result<()> {
        use SetRequest as SR;

        self.send_buff[0] = 0x01;
        self.send_buff[1] = request.to_u8();
        match request {
            SR::Toggle | SR::TurnOn | SR::TurnOff | SR::SaveSettings => {
                self.send_with_timeout(2).await?;
            }
            SR::Settings(settings) => {
                let settings_string =
                    serde_json::to_string(&settings).map_err(Error::SerializeJson)?;
                self.send_json_string(&settings_string).await?;
            }
            SR::WifiSettings(settings) => {
                let settings_string =
                    serde_json::to_string(&settings).map_err(Error::SerializeJson)?;
                self.send_json_string(&settings_string).await?;
            }
            SR::CurrentPresetSettings(settings) => {
                let settings_string =
                    serde_json::to_string(&settings).map_err(Error::SerializeJson)?;
                self.send_json_string(&settings_string).await?;
            }
            SR::Preset(value) | SR::Brightness(value) | SR::Speed(value) | SR::Scale(value) => {
                self.send_u8(value).await?;
            }
        }
        Ok(())
    }

    async fn send_json_string(&mut self, json_string: &str) -> Result<()> {
        let json_bytes = json_string.as_bytes();
        self.send_buff[2..2 + json_bytes.len()].copy_from_slice(json_bytes);
        self.send_with_timeout(2 + json_bytes.len()).await?;
        Ok(())
    }

    async fn send_u8(&mut self, value: u8) -> Result<()> {
        self.send_buff[2] = value;
        self.send_with_timeout(3).await?;
        Ok(())
    }

    async fn send_with_timeout(&self, msg_len: usize) -> Result<()> {
        tokio::time::timeout(Duration::from_millis(500), self.send(msg_len))
            .await
            .map_err(|_| Error::FutureTimeout)??;
        Ok(())
    }

    async fn send(&self, msg_len: usize) -> Result<()> {
        match &self.device_addr {
            Some(addr) => {
                let _ = self
                    .socket
                    .send_to(&self.send_buff[..msg_len], addr)
                    .await
                    .map_err(Error::UdpSend)?;
            }
            None => log::warn!("Cannot send message: address unset"),
        }
        Ok(())
    }

    async fn set_device_addr(&mut self, addr: SocketAddr) {
        self.device_addr = Some(addr);
        if let Err(err) = self.socket.connect(addr).await.map_err(Error::UdpBind) {
            log::error!("{err}");
            return;
        }
        log::info!("Set socket addr to: {}", addr);
    }
}

struct Reciever {
    socket: Arc<UdpSocket>,
    recv_buff: [u8; 1024],
}

impl Reciever {
    fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket,
            recv_buff: [0; 1024],
        }
    }

    async fn recv(&mut self) -> Result<DeviceResponse> {
        use DeviceGetResponse as DGR;
        use DeviceResponse as DR;
        use DeviceSetResponse as DSR;

        let size = self
            .socket
            .recv(&mut self.recv_buff)
            .await
            .map_err(Error::UdpRecv)?;

        if self.recv_buff[0] != 1 {
            return Err(Error::InvalidProtocolVersion);
        }

        match self.recv_buff[1] {
            0x00 => Ok(DR::Error),
            0x01 => Ok(DR::Get(DGR::Ping)),
            0x02 => {
                let is_on = self.recv_buff[2] != 0;
                Ok(DR::Get(DGR::IsOn(is_on)))
            }
            0x03 => {
                let preset_id = self.recv_buff[2];
                Ok(DR::Get(DGR::CurrentPresetId(preset_id)))
            }
            0x04 => {
                let preset_info: Vec<Preset> = serde_json::from_slice(&self.recv_buff[2..size])
                    .map_err(Error::DeserializeJson)?;
                Ok(DR::Get(DGR::PresetInfo(preset_info)))
            }
            0x05 => {
                let settings: DeviceSettings = serde_json::from_slice(&self.recv_buff[2..size])
                    .map_err(Error::DeserializeJson)?;
                Ok(DR::Get(DGR::Settings(settings)))
            }
            0x06 => {
                let preset_settings: PresetSettings =
                    serde_json::from_slice(&self.recv_buff[2..size])
                        .map_err(Error::DeserializeJson)?;
                Ok(DR::Get(DGR::CurrentPresetSettings(preset_settings)))
            }
            0x07 => {
                let wifi_settings: DeviceWifiSettings =
                    serde_json::from_slice(&self.recv_buff[2..size])
                        .map_err(Error::DeserializeJson)?;
                Ok(DR::Get(DGR::WifiSettings(wifi_settings)))
            }
            0x08 => Ok(DR::Set(DSR::Toggle)),
            0x09 => Ok(DR::Set(DSR::TurnOn)),
            0x0A => Ok(DR::Set(DSR::TurnOff)),
            0x0B => Ok(DR::Set(DSR::Preset)),
            0x0C => Ok(DR::Set(DSR::Settings)),
            0x0D => Ok(DR::Set(DSR::WifiSettings)),
            0x0E => Ok(DR::Set(DSR::CurrentPresetSettings)),
            0x0F => Ok(DR::Set(DSR::Brightness)),
            0x10 => Ok(DR::Set(DSR::Speed)),
            0x11 => Ok(DR::Set(DSR::Scale)),
            0x12 => Ok(DR::Set(DSR::SaveSettings)),
            _ => Err(Error::UnknownResponseMethod),
        }
    }
}

pub fn connection_worker() -> impl Stream<Item = Response> {
    const CHANNEL_SIZE: usize = 100;

    stream::channel(CHANNEL_SIZE, |mut output| async move {
        let (tx, mut rx) = mpsc::channel(CHANNEL_SIZE);

        output
            .send(Response::Ready(tx))
            .await
            .expect("Error sending mpsc sender to main thread!");

        let socket = Arc::new(
            UdpSocket::bind("0.0.0.0:30462")
                .await
                .expect("Could not open UDP socket connection!"),
        );

        let reciever = Reciever::new(Arc::clone(&socket));
        let recv_task = tokio::spawn(recv_worker(reciever, output));

        let mut sender = Sender::new(socket);
        let send_task = tokio::spawn(async move {
            loop {
                let request = rx.select_next_some().await;
                if let Err(err) = sender.handle_request(request).await {
                    log::error!("{err}");
                }
            }
        });

        let _ = futures::future::join(recv_task, send_task).await;
    })
}

async fn recv_worker(mut reciever: Reciever, mut output: mpsc::Sender<Response>) -> ! {
    loop {
        if let Err(err) = process_recv_message_fallible(&mut reciever, &mut output).await {
            log::error!("{err}");
        }
    }
}

async fn process_recv_message_fallible(
    reciever: &mut Reciever,
    output: &mut mpsc::Sender<Response>,
) -> Result<()> {
    let response = reciever.recv().await?;
    output
        .send(Response::Device(response))
        .await
        .map_err(Error::MpscSend)
}
