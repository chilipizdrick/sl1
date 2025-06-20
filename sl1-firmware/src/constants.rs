use embassy_time::Duration;

pub const PRESET_COUNT: u8 = 4;
pub const LED_COUNT: usize = 79;
pub const LEDS_DATA_BUFFER_SIZE: usize = 12 * LED_COUNT + 40;
pub const FRAME_TIME: Duration = Duration::from_millis(20);
pub const RANDOM_SEED: u64 = 0x0123_4567_89ab_cdef;
pub const SERVER_PORT: u16 = 30462;
pub const MESSAGE_BUFFER_LENGTH: usize = 1024;
pub const MINIMAL_CLIENT_MESSAGE_LENGTH: usize = 2;
pub const SETTINGS_STORAGE_OFFSET: u32 = 0x310000;
pub const PRESET_INFO: &str = r#"[{"id":0,"name":"Static Color"},{"id":1,"name":"Dynamic Color"},{"id":2,"name":"Running Rainbow"},{"id":3,"name":"Fire"}]"#;
pub const DEFAULT_WIFI_SSID: &str = env!("WIFI_SSID");
pub const DEFAULT_WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
