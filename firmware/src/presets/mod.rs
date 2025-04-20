mod dynamic_color;
mod fire;
mod noise;
mod running_rainbow;
mod static_color;
mod utils;

use core::sync::atomic::Ordering;
use embassy_time::{Duration, Timer};
use smart_leds_trait::SmartLedsWrite;

use crate::{
    FRAME_TIME_MS, LED_COUNT, LedsAdapter, Result, SETTINGS, SHOULD_UPDATE,
    settings::PresetSettings,
};

trait Preset {
    async fn run(leds: &mut LedsAdapter, preset_settings: &PresetSettings) -> Result<()>;
}

pub async fn run_renderer(mut leds: LedsAdapter) -> ! {
    loop {
        SHOULD_UPDATE.store(false, Ordering::Relaxed);

        if !SETTINGS.get().lock().await.is_on {
            draw_black(&mut leds);
            loop {
                if SHOULD_UPDATE.load(Ordering::Relaxed) {
                    break;
                }
                Timer::after(Duration::from_millis(FRAME_TIME_MS)).await;
            }
            continue;
        }

        let settings_lock = SETTINGS.get().lock().await;
        let preset_id = settings_lock.current_preset_id;
        let preset_settings = settings_lock.preset_settings[preset_id.id() as usize];
        drop(settings_lock);

        let res = match preset_id.id() {
            0 => static_color::StaticColorPreset::run(&mut leds, &preset_settings).await,
            1 => dynamic_color::DynamicColorPreset::run(&mut leds, &preset_settings).await,
            2 => running_rainbow::RunningRainbowPreset::run(&mut leds, &preset_settings).await,
            3 => fire::FirePreset::run(&mut leds, &preset_settings).await,
            _ => unreachable!(),
        };

        match res {
            Ok(_) => {}
            Err(e) => {
                log::error!("{e}")
            }
        }
    }
}

fn draw_black(leds: &mut LedsAdapter) {
    leds.write(core::iter::repeat_n([0, 0, 0], LED_COUNT))
        .unwrap();
}
