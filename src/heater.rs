use crate::{relay::RelayController, I2c0Bus, HEATER_POWER};
use defmt::{error, warn, Debug2Format};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embedded_hal_async::i2c::I2c;

async fn set_heater_relays<I2C, E>(
    relay_controller: &mut RelayController<I2C, E>,
    relay_2: bool,
    relay_3: bool,
    relay_4: bool,
) -> Result<(), crate::relay::Error<E>>
where
    I2C: I2c<Error = E>,
{
    if relay_2 {
        relay_controller.relay_on(2).await?;
    } else {
        relay_controller.relay_off(2).await?;
    }

    if relay_3 {
        relay_controller.relay_on(3).await?;
    } else {
        relay_controller.relay_off(3).await?;
    }

    if relay_4 {
        relay_controller.relay_on(4).await?;
    } else {
        relay_controller.relay_off(4).await?;
    }

    Ok(())
}

async fn turn_all_off_with_retry<I2C, E>(
    relay_controller: &mut RelayController<I2C, E>,
    max_retries: usize,
) -> Result<(), crate::relay::Error<E>>
where
    I2C: I2c<Error = E>,
    E: core::fmt::Debug,
{
    let mut attempts = 0;
    loop {
        match relay_controller.all_off().await {
            Ok(result) => return Ok(result),
            Err(err) if attempts < max_retries => {
                attempts += 1;
                warn!(
                    "Turning off relays failed (attempt {}/{}) with error {}; retrying...",
                    attempts,
                    max_retries + 1,
                    Debug2Format(&err)
                );
                embassy_time::Timer::after_millis(10).await;
            }
            Err(err) => return Err(err),
        }
    }
}

async fn set_fan_with_retry<I2C, E>(
    relay_controller: &mut RelayController<I2C, E>,
    on: bool,
    max_retries: usize,
) -> Result<(), crate::relay::Error<E>>
where
    I2C: I2c<Error = E>,
    E: core::fmt::Debug,
{
    let mut attempts = 0;
    loop {
        let result = if on {
            relay_controller.relay_on(1).await
        } else {
            relay_controller.relay_off(1).await
        };

        match result {
            Ok(()) => return Ok(()),
            Err(err) if attempts < max_retries => {
                attempts += 1;
                warn!(
                    "Fan command failed (attempt {}/{}) with error {}; retrying...",
                    attempts,
                    max_retries + 1,
                    Debug2Format(&err)
                );
                embassy_time::Timer::after_millis(10).await;
            }
            Err(err) => return Err(err),
        }
    }
}

#[embassy_executor::task]
pub async fn heater_task(i2c_bus: &'static I2c0Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut relay_controller = RelayController::new(i2c_dev);

    if let Err(e) = relay_controller.all_off().await {
        error!("Failed to initialize heater relays: {}", Debug2Format(&e));
        return;
    }

    let receiver = HEATER_POWER.receiver();

    loop {
        let command = receiver.receive().await;
        match command {
            crate::HeaterCommand::SetPower(power) => {
                let result = match power {
                    0 => set_heater_relays(&mut relay_controller, false, false, false).await,
                    1..=33 => set_heater_relays(&mut relay_controller, true, false, false).await,
                    34..=66 => set_heater_relays(&mut relay_controller, true, false, true).await,
                    67..=100 => set_heater_relays(&mut relay_controller, true, true, true).await,
                    _ => {
                        warn!("Invalid heater power level: {}", power);
                        continue;
                    }
                };

                if let Err(e) = result {
                    error!(
                        "Failed to set heater power to {}: {}",
                        power,
                        Debug2Format(&e)
                    );

                    let retry_result = turn_all_off_with_retry(&mut relay_controller, 2).await;

                    if let Err(retry_e) = retry_result {
                        error!(
                            "Failed to turn off heater relays after error: {}",
                            Debug2Format(&retry_e)
                        );
                    } else {
                        warn!("Successfully turned off heater relays after error recovery");
                    }
                }
            }
            crate::HeaterCommand::SetFan(on) => {
                let result = set_fan_with_retry(&mut relay_controller, on, 2).await;

                if let Err(e) = result {
                    error!("Failed to set fan to {}: {}", on, Debug2Format(&e));
                }
            }
        }
    }
}
