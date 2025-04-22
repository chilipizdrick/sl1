use core::sync::atomic::Ordering;
use embassy_time::Ticker;
use smart_leds_trait::SmartLedsWrite;

use crate::{
    Error, FRAME_TIME, LED_COUNT, LedsAdapter, Result, SHOULD_UPDATE,
    presets::{
        Preset,
        utils::{color_wheel, dim, whiten},
    },
    settings::PresetSettings,
};

pub struct DynamicColorPreset {}

impl Preset for DynamicColorPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(FRAME_TIME);
        let wait_cycles = if preset_settings.speed < 128 {
            128 - preset_settings.speed
        } else {
            1
        };
        let speed_mult = if preset_settings.speed >= 128 {
            preset_settings.speed - 127
        } else {
            1
        };

        loop {
            for i in 0..=255 as u8 {
                let wheel_pos = i.wrapping_mul(speed_mult);
                let color = dim(
                    &whiten(&color_wheel(wheel_pos), preset_settings.scale),
                    255 - preset_settings.brightness,
                );

                leds.write([color; LED_COUNT].into_iter())
                    .map_err(|_| Error::LedAdapterWriteError)?;

                for _ in 0..wait_cycles {
                    if SHOULD_UPDATE.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    ticker.next().await;
                }
            }
        }
    }
}
