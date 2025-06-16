#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error parsing address: {0}")]
    AddrParse(std::net::AddrParseError),
    #[error("error deserializing json: {0}")]
    DeserializeJson(serde_json::Error),
    #[error("error deserializing toml: {0}")]
    DeserializeToml(toml::de::Error),
    #[error("error reading file: {0}")]
    FileRead(std::io::Error),
    #[error("error converting byte sequence to utf-8 sting: {0}")]
    FromUtf8(std::string::FromUtf8Error),
    #[error("filesystem error: {0}")]
    Fs(String),
    #[error("error writing to filesystem: {0}")]
    FsWrite(std::io::Error),
    #[error("recieved message with invalid protocol version")]
    InvalidProtocolVersion,
    #[error("error loading config: config file does not exist")]
    MissingConfig,
    #[error("error sending data via mpsc: {0}")]
    MpscSend(iced::futures::channel::mpsc::SendError),
    #[error("error parsing port: {0}")]
    PortParse(std::num::ParseIntError),
    #[error("error serializing toml: {0}")]
    SerializeToml(toml::ser::Error),
    #[error("error serializing json: {0}")]
    SerializeJson(serde_json::Error),
    #[error("reached timeout while executing future")]
    FutureTimeout,
    #[error("error opening UDP socket: {0}")]
    UdpBind(std::io::Error),
    #[error("error recieving from UDP socket: {0}")]
    UdpRecv(std::io::Error),
    #[error("error sending to UDP socket: {0}")]
    UdpSend(std::io::Error),
    #[error("recieved device response with unknown method ID")]
    UnknownResponseMethod,
    #[error("error parsind ip nework string: {0}")]
    IpNetworkParse(ipnetwork::IpNetworkError),
}

pub type Result<T> = std::result::Result<T, Error>;
