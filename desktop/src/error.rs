#[derive(Debug)]
pub enum Error {
    AddrParse(std::net::AddrParseError),
    DeserializeJson(serde_json::Error),
    DeserializeToml(toml::de::Error),
    FileRead(std::io::Error),
    FromUtf8(std::string::FromUtf8Error),
    Fs(String),
    FsWrite(std::io::Error),
    InvalidProtocolVersion,
    MissingConfig,
    MpscSend(iced::futures::channel::mpsc::SendError),
    PortParse(std::num::ParseIntError),
    SerializeToml(toml::ser::Error),
    SerializeJson(serde_json::Error),
    Timeout(tokio::time::error::Elapsed),
    UdpConnect(std::io::Error),
    UdpRecv(std::io::Error),
    UdpSend(std::io::Error),
    UnknownResponseMethod,
    SemaphorAcquire(tokio::sync::AcquireError),
    IpNetworkParse(ipnetwork::IpNetworkError),
}

impl std::fmt::Display for self::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for self::Error {}

pub type Result<T> = std::result::Result<T, self::Error>;
