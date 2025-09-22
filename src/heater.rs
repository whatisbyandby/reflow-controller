use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Timer;

use crate::{relay::RelayController, I2c0Bus};

#[embassy_executor::task]
pub async fn heater_task(i2c_bus: &'static I2c0Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);

    let mut relay_controller = RelayController::new(i2c_dev);
    relay_controller.all_off().await.unwrap();

    let receiver = crate::HEATER_COMMAND_CHANNEL.receiver();

    loop {
        let power = receiver.receive().await;
        match power {
            0 => {
                relay_controller.relay_off(2).await.unwrap();
                relay_controller.relay_off(3).await.unwrap();
                relay_controller.relay_off(4).await.unwrap();
            }
            1..=30 => {
                relay_controller.relay_on(4).await.unwrap(); // Top Heater One on
                Timer::after_secs(1).await;
                relay_controller.relay_off(4).await.unwrap();
                Timer::after_secs(2).await;

                relay_controller.relay_on(3).await.unwrap(); // Top Heater One on
                Timer::after_secs(1).await;
                relay_controller.relay_off(3).await.unwrap();
                Timer::after_secs(2).await;
            }
            31..=60 => {
                relay_controller.relay_on(4).await.unwrap(); // Top Heater One on
                Timer::after_secs(1).await;
                relay_controller.relay_off(4).await.unwrap();
                Timer::after_secs(1).await;

                relay_controller.relay_on(3).await.unwrap(); // Top Heater One on
                Timer::after_secs(1).await;
                relay_controller.relay_off(3).await.unwrap();
                Timer::after_secs(1).await;
            }
            61..=100 => {
                relay_controller.relay_on(4).await.unwrap(); // Top Heater One on
                relay_controller.relay_on(3).await.unwrap(); // Bottom Heater One on
            }
            _ => {
                info!("Invalid power level: {}", power);
            }
        }
    }
}
