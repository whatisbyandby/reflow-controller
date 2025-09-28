use crate::{LedState, OutputCommand, OUTPUT_COMMAND_CHANNEL, SYSTEM_TICK_MILLIS};

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use embassy_time::Timer;

pub static LED_STATE: Watch<CriticalSectionRawMutex, LedState, 1> = Watch::new();

#[embassy_executor::task]
pub async fn output_task(spawner: Spawner) {
    loop {
        Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
    }
}
