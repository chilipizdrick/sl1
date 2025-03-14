#![no_std]
#![no_main]

mod constants;
mod error;
mod presets;
mod server;
mod settings;
mod types;
mod wifi;

use core::{str::FromStr, sync::atomic::AtomicBool};
use embassy_executor::Spawner;
use embassy_net::StackResources;
use embassy_sync::lazy_lock::LazyLock;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rmt::Rmt, spi::master::Config, time::RateExtU32};
use esp_println::dbg;
use esp_storage::FlashStorage;
use esp_wifi::{
    wifi::{new_with_config, ClientConfiguration, WifiDevice, WifiStaDevice},
    EspWifiController,
};
use settings::init_settings_storage;
use static_cell::StaticCell;

use crate::settings::Settings;
pub use crate::{
    constants::*,
    error::{Error, Result},
    types::*,
};

extern crate alloc;

static SHOULD_UPDATE: AtomicBool = AtomicBool::new(true);
static STORAGE: LazyLock<Mutex<FlashStorage>> =
    LazyLock::new(|| Mutex::new(FlashStorage::default()));
static SETTINGS: LazyLock<Mutex<Settings>> = LazyLock::new(|| Mutex::new(Settings::default()));

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(64 * 1024);

    init_settings_storage().await.unwrap();
    *SETTINGS.get().lock().await = Settings::load().await.unwrap();

    dbg!(SETTINGS.get().lock().await);

    let peripherals_config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(peripherals_config);

    esp_println::logger::init_logger_from_env();

    let timgsys = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timgsys.alarm0);

    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();
    let wifi_controller = ESP_WIFI_CONTROLLER.init(
        esp_wifi::init(
            timg0.timer0,
            esp_hal::rng::Rng::new(peripherals.RNG),
            peripherals.RADIO_CLK,
        )
        .unwrap(),
    );
    let sta_config = ClientConfiguration {
        ssid: heapless::String::from_str(WIFI_SSID).unwrap(),
        password: heapless::String::from_str(WIFI_PASSWORD).unwrap(),
        ..Default::default()
    };
    let (device, controller): (WifiDevice<'_, WifiStaDevice>, _) =
        new_with_config(wifi_controller, peripherals.WIFI, sta_config).unwrap();

    let dhcp_config = embassy_net::DhcpConfig::default();
    let net_config = embassy_net::Config::dhcpv4(dhcp_config);
    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        device,
        net_config,
        RESOURCES.init(StackResources::<4>::new()),
        RANDOM_SEED,
    );

    // let mut led_buffer = [0u8; 12 * LED_COUNT + 40];
    // let leds = ws2812_spi::prerendered::Ws2812::new(
    //     esp_hal::spi::master::Spi::new(peripherals.SPI2, Config::default().with_frequency(3.MHz()))
    //         .unwrap()
    //         .with_sck(peripherals.GPIO21)
    //         .with_miso(peripherals.GPIO20)
    //         .with_mosi(peripherals.GPIO10),
    //     &mut led_buffer,
    // );

    let rmt = Rmt::new(peripherals.RMT, 80.MHz()).unwrap();
    let led_strip_pin = peripherals.GPIO10;

    spawner.spawn(crate::wifi::wifi_task(controller)).unwrap();
    spawner.spawn(crate::server::net_task(runner)).unwrap();
    spawner.spawn(crate::server::server_task(stack)).unwrap();

    crate::presets::run_renderer(rmt, led_strip_pin).await;
}
