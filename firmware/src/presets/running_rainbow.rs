use core::sync::atomic::Ordering;
use embassy_time::{Duration, Ticker};
use smart_leds_trait::SmartLedsWrite;

use crate::{
    Error, LED_COUNT, LedsAdapter, Result, SHOULD_UPDATE,
    presets::{
        Preset,
        utils::{color_wheel, dim},
    },
    settings::PresetSettings,
};

pub struct RunningRainbowPreset {}

impl Preset for RunningRainbowPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(Duration::from_millis(258 - preset_settings.speed as u64));

        let mut strip = [[0; 3]; LED_COUNT];

        loop {
            for i in 0..=255 {
                #[allow(clippy::needless_range_loop)]
                for idx in 0..LED_COUNT {
                    let wheel_pos = ((idx * 256 * (preset_settings.scale as usize + 1) / LED_COUNT
                        % 256) as u8)
                        .wrapping_add(i);
                    strip[idx] = dim(&color_wheel(wheel_pos), 255 - preset_settings.brightness);
                }

                leds.write(strip.into_iter())
                    .map_err(|_| Error::LedAdapterWriteError)?;

                if SHOULD_UPDATE.load(Ordering::Relaxed) {
                    return Ok(());
                }

                ticker.next().await;
            }
        }
    }
}
