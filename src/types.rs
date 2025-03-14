use esp_hal_smartled::SmartLedsAdapter;

use crate::SMART_LEDS_BUFFER_SIZE;

pub type Mutex<T> = embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, T>;
pub type LedsAdapter =
    SmartLedsAdapter<esp_hal::rmt::Channel<esp_hal::Blocking, 0>, SMART_LEDS_BUFFER_SIZE>;
