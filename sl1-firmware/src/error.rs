#[derive(Debug)]
pub enum Error {
    PresetIdOutOfBounds,
    UnableToLockMutex,
    LedAdapterWrite,
    UnsupportedClientMessageMethod,
    ProtocolVersion(sl1_protocol::VersionError),
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    SendError(embassy_net::udp::SendError),
    StorageWrite(esp_storage::FlashStorageError),
    StorageRead(esp_storage::FlashStorageError),
    Unspecified,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type Result<T> = core::result::Result<T, Error>;
