use core::sync::atomic::Ordering;
use embassy_time::{Duration, Ticker};
use smart_leds_trait::SmartLedsWrite;

use crate::{
    presets::{utils::color_wheel, Preset},
    settings::PresetSettings,
    Error, LedsAdapter, Result, FRAME_TIME_MS, LED_COUNT, SHOULD_UPDATE,
};

pub struct RunningRainbowPreset {}

impl Preset for RunningRainbowPreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(Duration::from_millis(FRAME_TIME_MS));

        let mut strip = [[0; 3]; LED_COUNT];

        loop {
            for i in 0..=255 {
                for _ in 0..=(255 - preset_settings.speed) {
                    strip.iter_mut().enumerate().for_each(|(led_idx, led)| {
                        let wheel_pos = (((led_idx * 256 / LED_COUNT)
                            * (preset_settings.scale as usize + 1)
                            + i)
                            % 256) as u8;
                        color_wheel(&wheel_pos).into_iter().enumerate().for_each(
                            |(val_idx, val)| {
                                led[val_idx] =
                                    (val as u16 * preset_settings.brightness as u16 / 255) as u8
                            },
                        );
                    });

                    leds.write(strip.into_iter())
                        .map_err(Error::LedAdapterWriteError)?;

                    if SHOULD_UPDATE.load(Ordering::Relaxed) {
                        return Ok(());
                    }

                    ticker.next().await;
                }
            }
        }
    }
}
