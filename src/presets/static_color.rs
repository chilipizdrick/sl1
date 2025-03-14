use core::sync::atomic::Ordering;
use embassy_time::{Duration, Ticker};
use smart_leds_trait::SmartLedsWrite;

use crate::{
    presets::{
        utils::{color_wheel, whiten},
        Preset,
    },
    settings::PresetSettings,
    Error, LedsAdapter, Result, FRAME_TIME_MS, LED_COUNT, SHOULD_UPDATE,
};

pub struct StaticColorPreset {}

impl Preset for StaticColorPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(Duration::from_millis(FRAME_TIME_MS));

        let color = whiten(&color_wheel(&preset_settings.scale), preset_settings.speed);

        loop {
            leds.write([color; LED_COUNT].into_iter())
                .map_err(Error::LedAdapterWriteError)?;

            if SHOULD_UPDATE.load(Ordering::Relaxed) {
                return Ok(());
            }

            ticker.next().await;
        }
    }
}
