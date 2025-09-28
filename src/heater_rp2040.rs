#[path = "./relay.rs"]
mod relay;

use crate::{resources_rp2040::I2c0Bus, HEATER_POWER, SYSTEM_TICK_MILLIS};
use defmt::{error, info, warn, Debug2Format};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

async fn set_heater_relays<I2C, E>(
    relay_controller: &mut relay::RelayController<I2C, E>,
    relay_2: bool,
    relay_3: bool,
    relay_4: bool,
) -> Result<(), relay::Error<E>>
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

#[derive(Clone, Copy)]
struct RelaySchedule {
    relay_2: [bool; 10],
    relay_3: [bool; 10],
    relay_4: [bool; 10],
}

impl RelaySchedule {
    fn new() -> Self {
        Self {
            relay_2: [false; 10],
            relay_3: [false; 10],
            relay_4: [false; 10],
        }
    }

    fn calculate_for_power(power: u8, rotation: u8) -> Self {
        let mut schedule = Self::new();

        if power == 0 {
            return schedule;
        }

        // Convert power (0-100) to total relay-time units needed
        // Each relay represents 33.33% power, so 3 relays = 100%
        // We have 10 time slots of 100ms each
        let total_relay_time = (power as f32 / 100.0) * 30.0; // 30 = 3 relays * 10 time slots

        // Calculate how many full relays (10 slots each) and partial relay time
        let full_relays = (total_relay_time as u8) / 10;
        let partial_slots = (total_relay_time as u8) % 10;

        // Determine which relay is the "active" (cycling) relay based on rotation
        let active_relay = (rotation % 3) + 2; // Cycles through relays 2, 3, 4

        // Helper function to set all slots for a relay
        let set_relay_slots = |_relay_num: u8, slots: u8| -> [bool; 10] {
            let mut relay_schedule = [false; 10];
            for i in 0..(slots as usize).min(10) {
                relay_schedule[i] = true;
            }
            relay_schedule
        };

        match full_relays {
            0 => {
                // Less than 33% power - only active relay cycles
                match active_relay {
                    2 => {
                        schedule.relay_2 = set_relay_slots(2, partial_slots);
                        schedule.relay_3 = [false; 10];
                        schedule.relay_4 = [false; 10];
                    }
                    3 => {
                        schedule.relay_2 = [false; 10];
                        schedule.relay_3 = set_relay_slots(3, partial_slots);
                        schedule.relay_4 = [false; 10];
                    }
                    4 => {
                        schedule.relay_2 = [false; 10];
                        schedule.relay_3 = [false; 10];
                        schedule.relay_4 = set_relay_slots(4, partial_slots);
                    }
                    _ => unreachable!(),
                }
            }
            1 => {
                // 33-66% power - one relay full on, active relay cycles
                match active_relay {
                    2 => {
                        schedule.relay_2 = set_relay_slots(2, partial_slots);
                        schedule.relay_3 = [true; 10];
                        schedule.relay_4 = [false; 10];
                    }
                    3 => {
                        schedule.relay_2 = [true; 10];
                        schedule.relay_3 = set_relay_slots(3, partial_slots);
                        schedule.relay_4 = [false; 10];
                    }
                    4 => {
                        schedule.relay_2 = [true; 10];
                        schedule.relay_3 = [false; 10];
                        schedule.relay_4 = set_relay_slots(4, partial_slots);
                    }
                    _ => unreachable!(),
                }
            }
            2 => {
                // 66-100% power - two relays full on, active relay cycles
                match active_relay {
                    2 => {
                        schedule.relay_2 = set_relay_slots(2, partial_slots);
                        schedule.relay_3 = [true; 10];
                        schedule.relay_4 = [true; 10];
                    }
                    3 => {
                        schedule.relay_2 = [true; 10];
                        schedule.relay_3 = set_relay_slots(3, partial_slots);
                        schedule.relay_4 = [true; 10];
                    }
                    4 => {
                        schedule.relay_2 = [true; 10];
                        schedule.relay_3 = [true; 10];
                        schedule.relay_4 = set_relay_slots(4, partial_slots);
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                // 100% power - all relays full on
                schedule.relay_2 = [true; 10];
                schedule.relay_3 = [true; 10];
                schedule.relay_4 = [true; 10];
            }
        }

        schedule
    }
}

async fn run_power_cycle<I2C, E>(
    relay_controller: &mut relay::RelayController<I2C, E>,
    schedule: RelaySchedule,
) -> Result<(), relay::Error<E>>
where
    I2C: I2c<Error = E>,
    E: core::fmt::Debug,
{
    for slot in 0..10 {
        // Set relay states for this 100ms slot
        let result = set_heater_relays(
            relay_controller,
            schedule.relay_2[slot],
            schedule.relay_3[slot],
            schedule.relay_4[slot],
        )
        .await;

        result?;

        // Wait for 100ms before next slot
        Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
    }

    Ok(())
}

async fn turn_all_off_with_retry<I2C, E>(
    relay_controller: &mut relay::RelayController<I2C, E>,
    max_retries: usize,
) -> Result<(), relay::Error<E>>
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
                Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
            }
            Err(err) => return Err(err),
        }
    }
}

async fn set_fan_with_retry<I2C, E>(
    relay_controller: &mut relay::RelayController<I2C, E>,
    on: bool,
    max_retries: usize,
) -> Result<(), relay::Error<E>>
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
                Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
            }
            Err(err) => return Err(err),
        }
    }
}

#[cfg(feature = "rp2040")]
#[embassy_executor::task]
pub async fn heater_task(i2c_bus: &'static I2c0Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut relay_controller = relay::RelayController::new(i2c_dev);

    if let Err(e) = relay_controller.all_off().await {
        error!("Failed to initialize heater relays: {}", Debug2Format(&e));
        return;
    }

    let receiver = HEATER_POWER.receiver();

    let mut current_power = 0u8;
    let mut rotation_counter = 0u8;
    let mut last_schedule = RelaySchedule::new();

    loop {
        // Check for new power commands (non-blocking)
        match receiver.try_receive() {
            Ok(command) => match command {
                crate::HeaterCommand::SetPower(power) => {
                    if power > 100 {
                        warn!("Invalid heater power level: {}", power);
                    } else if power != current_power {
                        current_power = power;
                        rotation_counter = rotation_counter.wrapping_add(1);
                        last_schedule = RelaySchedule::calculate_for_power(power, rotation_counter);
                    }
                }
                crate::HeaterCommand::SetFan(on) => {
                    let result = set_fan_with_retry(&mut relay_controller, on, 2).await;

                    if let Err(e) = result {
                        error!("Failed to set fan to {}: {}", on, Debug2Format(&e));
                    }
                }
                crate::HeaterCommand::SimulationReset => {
                    info!("Resetting heater simulation state");
                    current_power = 0;
                    rotation_counter = 0;
                    last_schedule = RelaySchedule::new();
                    // Turn off all relays
                    let result =
                        set_heater_relays(&mut relay_controller, false, false, false).await;
                    if let Err(e) = result {
                        error!(
                            "Failed to turn off heater relays during reset: {}",
                            Debug2Format(&e)
                        );
                    }
                }
                crate::HeaterCommand::UpdatePidParameters { kp, ki, kd } => {
                    info!("PID parameters updated: Kp={}, Ki={}, Kd={}", kp, ki, kd);
                    // Note: Actual PID controller is updated in reflow_controller.rs
                    // This is just for logging at the heater task level
                }
            },
            Err(_) => {} // No new command, continue with current power level
        }

        // Run the power cycle for current power level
        if current_power > 0 {
            let result = run_power_cycle(&mut relay_controller, last_schedule).await;

            if let Err(e) = result {
                error!(
                    "Failed to run power cycle at {}%: {}",
                    current_power,
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

                // Reset to 0 power after error
                current_power = 0;
                last_schedule = RelaySchedule::new();
            }
        } else {
            // Power is 0, ensure all relays are off and wait
            let result = set_heater_relays(&mut relay_controller, false, false, false).await;
            if let Err(e) = result {
                error!("Failed to turn off heater relays: {}", Debug2Format(&e));
            }
            Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
        }
    }
}
