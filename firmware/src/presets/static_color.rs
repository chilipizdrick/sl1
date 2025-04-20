use core::sync::atomic::Ordering;
use embassy_time::{Duration, Ticker};
use smart_leds_trait::SmartLedsWrite;

use crate::{
    Error, FRAME_TIME_MS, LED_COUNT, LedsAdapter, Result, SHOULD_UPDATE,
    presets::{
        Preset,
        utils::{color_wheel, dim, whiten},
    },
    settings::PresetSettings,
};

pub struct StaticColorPreset {}

impl Preset for StaticColorPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(Duration::from_millis(FRAME_TIME_MS));

        let color = dim(
            &whiten(&color_wheel(preset_settings.scale), preset_settings.speed),
            255 - preset_settings.brightness,
        );

        loop {
            leds.write([color; LED_COUNT].into_iter())
                .map_err(|_| Error::LedAdapterWriteError)?;

            if SHOULD_UPDATE.load(Ordering::Relaxed) {
                return Ok(());
            }

            ticker.next().await;
        }
    }
}
