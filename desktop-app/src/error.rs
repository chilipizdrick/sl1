#[derive(Debug)]
pub enum Error {
    DeviceConnection(std::io::Error),
    DeviceSend(std::io::Error),
    DeviceRecieve(std::io::Error),
    Serialization,
    Deserialization,
    FromUtf8(std::string::FromUtf8Error),
    Fs,
    MissingConfig,
    IncompleteStateBuilder,
    AddrParse(std::net::AddrParseError),
    PortParse(std::num::ParseIntError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;
