use embassy_time::{Duration, Timer};
use esp_wifi::wifi::{WifiController, WifiEvent};

use crate::DEFAULT_WIFI_SSID;

#[embassy_executor::task]
pub async fn wifi_task(mut controller: WifiController<'static>) -> ! {
    controller.start_async().await.unwrap();
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                log::info!(target: "WIFI", "Connected to {} wifi network.", DEFAULT_WIFI_SSID);
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                log::info!(target: "WIFI", "Disconnected from network, reconnecting...");
            }

            Err(err) => {
                log::error!(target: "WIFI", "Error connecting to wifi network: {err:?}.\nReconnecting...");
                Timer::after(Duration::from_millis(5000)).await;
            }
        };
    }
}
