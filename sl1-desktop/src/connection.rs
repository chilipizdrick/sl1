use iced::{
    futures::{Stream, StreamExt, channel::mpsc, sink::SinkExt},
    stream,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::UdpSocket;

use crate::{
    Error, Result,
    device::{DeviceSettings, DeviceWifiSettings, Preset, PresetId, PresetSettings},
};

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
        match self {
            GetRequest::Ping => 0x01,
            GetRequest::IsOn => 0x02,
            GetRequest::CurrentPresetId => 0x03,
            GetRequest::PresetInfo => 0x04,
            GetRequest::Settings => 0x05,
            GetRequest::WifiSettings => 0x06,
            GetRequest::CurrentPresetSettings => 0x07,
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
}

impl SetRequest {
    fn to_u8(&self) -> u8 {
        match self {
            SetRequest::Toggle => 0x08,
            SetRequest::TurnOn => 0x09,
            SetRequest::TurnOff => 0x0A,
            SetRequest::Preset(_) => 0x0B,
            SetRequest::Settings(_) => 0x0C,
            SetRequest::WifiSettings(_) => 0x0D,
            SetRequest::CurrentPresetSettings(_) => 0x0E,
            SetRequest::Brightness(_) => 0x0F,
            SetRequest::Speed(_) => 0x10,
            SetRequest::Scale(_) => 0x11,
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
        log::debug!("{:?}", &request);
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
        self.send_buff[0] = 0x01;
        self.send_buff[1] = request.to_u8();
        match request {
            SetRequest::Toggle | SetRequest::TurnOn | SetRequest::TurnOff => {
                self.send_with_timeout(2).await?;
            }
            SetRequest::Settings(settings) => {
                let settings_string =
                    serde_json::to_string(&settings).map_err(Error::SerializeJson)?;
                self.send_json_string(&settings_string).await?;
            }
            SetRequest::WifiSettings(settings) => {
                let settings_string =
                    serde_json::to_string(&settings).map_err(Error::SerializeJson)?;
                self.send_json_string(&settings_string).await?;
            }
            SetRequest::CurrentPresetSettings(settings) => {
                let settings_string =
                    serde_json::to_string(&settings).map_err(Error::SerializeJson)?;
                self.send_json_string(&settings_string).await?;
            }
            SetRequest::Preset(value)
            | SetRequest::Brightness(value)
            | SetRequest::Speed(value)
            | SetRequest::Scale(value) => {
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
            .map_err(Error::Timeout)??;
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
        if let Err(err) = self.socket.connect(addr).await.map_err(Error::UdpConnect) {
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
        let size = self
            .socket
            .recv(&mut self.recv_buff)
            .await
            .map_err(Error::UdpRecv)?;

        if self.recv_buff[0] != 1 {
            return Err(Error::InvalidProtocolVersion);
        }

        match self.recv_buff[1] {
            0x00 => Ok(DeviceResponse::Error),
            0x01 => Ok(DeviceResponse::Get(DeviceGetResponse::Ping)),
            0x02 => {
                let is_on = self.recv_buff[2] != 0;
                Ok(DeviceResponse::Get(DeviceGetResponse::IsOn(is_on)))
            }
            0x03 => {
                let preset_id = self.recv_buff[2];
                Ok(DeviceResponse::Get(DeviceGetResponse::CurrentPresetId(
                    preset_id,
                )))
            }
            0x04 => {
                let preset_info: Vec<Preset> = serde_json::from_slice(&self.recv_buff[2..size])
                    .map_err(Error::DeserializeJson)?;
                Ok(DeviceResponse::Get(DeviceGetResponse::PresetInfo(
                    preset_info,
                )))
            }
            0x05 => {
                let settings: DeviceSettings = serde_json::from_slice(&self.recv_buff[2..size])
                    .map_err(Error::DeserializeJson)?;
                Ok(DeviceResponse::Get(DeviceGetResponse::Settings(settings)))
            }
            0x06 => {
                let preset_settings: PresetSettings =
                    serde_json::from_slice(&self.recv_buff[2..size])
                        .map_err(Error::DeserializeJson)?;
                Ok(DeviceResponse::Get(
                    DeviceGetResponse::CurrentPresetSettings(preset_settings),
                ))
            }
            0x07 => {
                let wifi_settings: DeviceWifiSettings =
                    serde_json::from_slice(&self.recv_buff[2..size])
                        .map_err(Error::DeserializeJson)?;
                Ok(DeviceResponse::Get(DeviceGetResponse::WifiSettings(
                    wifi_settings,
                )))
            }
            0x08 => Ok(DeviceResponse::Set(DeviceSetResponse::Toggle)),
            0x09 => Ok(DeviceResponse::Set(DeviceSetResponse::TurnOn)),
            0x0A => Ok(DeviceResponse::Set(DeviceSetResponse::TurnOff)),
            0x0B => Ok(DeviceResponse::Set(DeviceSetResponse::Preset)),
            0x0C => Ok(DeviceResponse::Set(DeviceSetResponse::Settings)),
            0x0D => Ok(DeviceResponse::Set(DeviceSetResponse::WifiSettings)),
            0x0E => Ok(DeviceResponse::Set(
                DeviceSetResponse::CurrentPresetSettings,
            )),
            0x0F => Ok(DeviceResponse::Set(DeviceSetResponse::Brightness)),
            0x10 => Ok(DeviceResponse::Set(DeviceSetResponse::Speed)),
            0x11 => Ok(DeviceResponse::Set(DeviceSetResponse::Scale)),
            _ => Err(Error::UnknownResponseMethod),
        }
    }
}

pub fn connection_worker() -> impl Stream<Item = Response> {
    const CHANNEL_SIZE: usize = 100;

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

    async fn recv_worker(mut reciever: Reciever, mut output: mpsc::Sender<Response>) -> ! {
        loop {
            if let Err(err) = process_recv_message_fallible(&mut reciever, &mut output).await {
                log::error!("{err}");
            }
        }
    }

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
        tokio::spawn(recv_worker(reciever, output));

        let mut sender = Sender::new(socket);
        loop {
            let request = rx.select_next_some().await;
            if let Err(err) = sender.handle_request(request).await {
                log::error!("{err}");
            }
        }
    })
}
