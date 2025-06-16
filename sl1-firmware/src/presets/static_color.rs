use core::sync::atomic::Ordering;

use embassy_time::Ticker;
use smart_leds_trait::SmartLedsWrite;

use crate::presets::Preset;
use crate::presets::utils::{color_wheel, dim, whiten};
use crate::settings::PresetSettings;
use crate::{Error, FRAME_TIME, LED_COUNT, LedsAdapter, Result, SHOULD_UPDATE};

pub struct StaticColorPreset {}

impl Preset for StaticColorPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(FRAME_TIME);

        let color = dim(
            &whiten(&color_wheel(preset_settings.scale), preset_settings.speed),
            255 - preset_settings.brightness,
        );

        loop {
            leds.write([color; LED_COUNT].into_iter())
                .map_err(|_| Error::LedAdapterWrite)?;

            if SHOULD_UPDATE.load(Ordering::Relaxed) {
                return Ok(());
            }

            ticker.next().await;
        }
    }
}
