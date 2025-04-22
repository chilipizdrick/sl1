use core::sync::atomic::Ordering;
use embassy_time::Ticker;
use smart_leds_trait::SmartLedsWrite;

use crate::{
    Error, FRAME_TIME, LED_COUNT, LedsAdapter, Result, SHOULD_UPDATE,
    presets::{Preset, noise::PerlinNoise},
    settings::PresetSettings,
};

use super::utils::lerp_gradient;

const PALETTE: [[u8; 3]; 5] = [[0, 0, 0], [127, 0, 0], [255, 0, 0], [255, 127, 0], [
    255, 255, 0,
]];

pub struct FirePreset {}

impl Preset for FirePreset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()> {
        let mut ticker = Ticker::every(FRAME_TIME);

        let mut strip = [[0; 3]; LED_COUNT];
        let mut time = 0u16;
        let perlin = PerlinNoise::default();
        let speed_mult = if preset_settings.speed > 0 {
            (preset_settings.speed / 8).clamp(1, 31)
        } else {
            0
        } as u16;

        loop {
            strip.iter_mut().enumerate().for_each(|(led_idx, led)| {
                let noise = perlin.get_u8_2d(led_idx as u16 * preset_settings.scale as u16, time);
                *led = lerp_gradient(&PALETTE, noise)
                    .map(|v: u8| (v as u16 * preset_settings.brightness as u16 / 255) as u8);
            });

            time = time.wrapping_add(speed_mult);

            leds.write(strip.into_iter())
                .map_err(|_| Error::LedAdapterWriteError)?;

            if SHOULD_UPDATE.load(Ordering::Relaxed) {
                return Ok(());
            }

            ticker.next().await;
        }
    }
}
