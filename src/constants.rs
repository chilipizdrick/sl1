pub const PRESET_COUNT: usize = 4;
pub const LED_COUNT: usize = 18;
pub const SMART_LEDS_BUFFER_SIZE: usize = 24 * LED_COUNT + 1;
pub const FRAME_TIME_MS: u64 = 10;
pub const RANDOM_SEED: u64 = 0x0123_4567_89ab_cdef;
pub const SERVER_PORT: u16 = 8080;
pub const UDP_MESSAGE_BUFFER_LENGTH: usize = 256;
pub const MINIMAL_CLIENT_MESSAGE_LENGTH: usize = 3;
pub const SETTINGS_STORAGE_OFFSET: u32 = 0x310000;
pub const PRESET_INFO: &str = r#"[{"name":"Static Color"},{"name":"Dynamic Color"},{"name":"Running Rainbow"},{"name":"Fire"}]"#;
pub const PONG: &str = "Pong!";
pub const WIFI_SSID: &str = env!("WIFI_SSID");
pub const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
