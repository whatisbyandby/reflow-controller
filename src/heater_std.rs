use crate::log::*;
use crate::SYSTEM_TICK_MILLIS;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn heater_task() {
    info!("Starting heater task");
    loop {
        Timer::after_millis((SYSTEM_TICK_MILLIS * 10).into()).await;
    }
}
