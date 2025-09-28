use embassy_executor::Spawner;

use embassy_time::Timer;

use crate::{Event, INPUT_EVENT_CHANNEL, SYSTEM_TICK_MILLIS};

#[embassy_executor::task]
pub async fn interface_task(spawner: Spawner) {
    Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
}
