pub type Mutex<T> = embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex, T>;

pub type LedsAdapter = ws2812_spi::prerendered::Ws2812<
    'static,
    esp_hal::spi::master::SpiDmaBus<'static, esp_hal::Blocking>,
>;
