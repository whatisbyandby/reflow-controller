use crate::log::*;
use crate::SYSTEM_TICK_MILLIS;
use embassy_executor::Spawner;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn usb_task(spawner: Spawner) {
    info!("Starting usb task");
    loop {
        Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
    }
}
