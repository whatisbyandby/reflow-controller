use embassy_executor::Spawner;

use embassy_time::Timer;

use crate::{Event, INPUT_EVENT_CHANNEL, SYSTEM_TICK_MILLIS};

#[embassy_executor::task]
pub async fn interface_task(_spawner: Spawner) {
    // Wait for initialization
    Timer::after_millis((SYSTEM_TICK_MILLIS * 20).into()).await;

    // Simulate door closed for testing
    INPUT_EVENT_CHANNEL
        .sender()
        .send(Event::DoorStateChanged(true))
        .await;

    log::info!("Door closed (simulated)");

    // Wait a bit more, then auto-start the reflow process
    Timer::after_millis((SYSTEM_TICK_MILLIS * 10).into()).await;

    INPUT_EVENT_CHANNEL
        .sender()
        .send(Event::StartCommand)
        .await;

    log::info!("Reflow process started automatically");

    // Keep task alive
    loop {
        Timer::after_millis((SYSTEM_TICK_MILLIS * 100).into()).await;
    }
}
