use core::sync::atomic::Ordering;

use embassy_time::Ticker;
use smart_leds_trait::SmartLedsWrite;

use crate::presets::Preset;
use crate::presets::utils::{color_wheel, dim};
use crate::settings::PresetSettings;
use crate::{Error, FRAME_TIME, LED_COUNT, LedsAdapter, Result, SHOULD_UPDATE};

pub struct RunningRainbowPreset {}

impl Preset for RunningRainbowPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(FRAME_TIME);
        let mut strip = [[0; 3]; LED_COUNT];
        let speed_mult = 128u8.wrapping_sub(preset_settings.speed);
        let scale_factor: usize = preset_settings.scale as usize * 2;

        loop {
            for i in 0..=255 as u8 {
                let frame_wheel_pos = i.wrapping_mul(speed_mult);
                #[allow(clippy::needless_range_loop)]
                for idx in 0..LED_COUNT {
                    let wheel_pos = ((idx * scale_factor / LED_COUNT % 256) as u8)
                        .wrapping_add(frame_wheel_pos);
                    strip[idx] = dim(&color_wheel(wheel_pos), 255 - preset_settings.brightness);
                }

                leds.write(strip.into_iter())
                    .map_err(|_| Error::LedAdapterWrite)?;

                if SHOULD_UPDATE.load(Ordering::Relaxed) {
                    return Ok(());
                }

                ticker.next().await;
            }
        }
    }
}
