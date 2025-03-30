#[derive(Debug)]
pub enum Error {
    PresetIdOutOfBounds,
    UnableToLockMutex,
    LedAdapterWriteError,
    UnsupportedClientMessageMethod,
    InvalidProtocolVersion,
    UnsupportedProtocolVersion,
    SerializationError(serde_json::Error),
    DeserializationError(serde_json::Error),
    SendError(embassy_net::tcp::Error),
    StorageWriteError(esp_storage::FlashStorageError),
    StorageReadError(esp_storage::FlashStorageError),
    Unspecified,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type Result<T> = core::result::Result<T, Error>;
