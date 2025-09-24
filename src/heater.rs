use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Timer;

use crate::{
    heater,
    relay::{self, RelayController},
    I2c0Bus, HEATER_POWER,
};

const NUM_MILLIS: u32 = 10_000; // 10 seconds

#[embassy_executor::task]
pub async fn heater_task(i2c_bus: &'static I2c0Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut relay_controller = RelayController::new(i2c_dev);
    relay_controller.all_off().await.unwrap();

    let mut receiver = HEATER_POWER.receiver().unwrap();

    loop {
        let power = receiver.changed().await;

        match power {
            0 => {
                relay_controller.relay_off(2).await;
                relay_controller.relay_off(3).await;
                relay_controller.relay_off(4).await;
            }
            1..=33 => {
                let total_on_time = (NUM_MILLIS as u32 * power as u32) / 100;
                let on_time = total_on_time / 3;
                let total_off_time = NUM_MILLIS - total_on_time;
                let off_time = total_off_time / 3;
                // 33% power means each of the 3 relays is on for 1/3 of the time
                // loop over 2-4 relays and turn them on for a fraction of the time
                for relay in 2..=4 {
                    relay_controller.relay_on(relay).await;
                    Timer::after_millis(on_time as u64).await;
                    relay_controller.relay_off(relay).await;
                    Timer::after_millis((off_time) as u64).await;
                }
            }
            34..=66 => {}
            67..=99 => {}
            100 => {
                relay_controller.relay_on(2).await;
                relay_controller.relay_on(3).await;
                relay_controller.relay_on(4).await;
            }
            _ => {}
        }
    }
}
