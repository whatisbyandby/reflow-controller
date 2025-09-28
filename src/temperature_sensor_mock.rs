use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub static CURRENT_TEMPERATURE: Signal<CriticalSectionRawMutex, f32> = Signal::new();
use crate::log::*;
use crate::SYSTEM_TICK_MILLIS;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn run_temperature_sensor() -> ! {
    use crate::HeaterCommand;
    use crate::HEATER_POWER;

    // info!("Starting mock temperature sensor with thermal simulation");

    // Thermal simulation parameters - configurable for testing
    let mut current_temp = 25.0; // Start at room temperature
    let ambient_temp = 25.0;
    let max_heating_rate = 3.0; // degrees C/second at 100% power (as requested)
    let thermal_mass = 0.3; // Factor affecting heat retention (0-1)
    let heat_loss_coefficient = 0.1; // Heat loss to ambient per degree difference
    let update_interval_ms = SYSTEM_TICK_MILLIS * 5;

    let time_step = update_interval_ms as f32 / SYSTEM_TICK_MILLIS as f32 / 10.0;

    info!(
        "Thermal parameters: max_rate={}°C/s, mass={}, loss={}",
        max_heating_rate, thermal_mass, heat_loss_coefficient
    );

    let mut fan_enabled = false;

    let heater_receiver = HEATER_POWER.receiver();
    let mut current_heater_power: u32 = 0;
    loop {
        // Check for heater power updates
        let new_command = heater_receiver.receive().await;
        match new_command {
            HeaterCommand::SetPower(p) => current_heater_power = p as u32,
            HeaterCommand::SetFan(on) => fan_enabled = on,
            HeaterCommand::SimulationReset => {
                // info!("Resetting thermal simulation to initial state");
                current_temp = 25.0; // Reset to room temperature
                fan_enabled = false;
                current_heater_power = 0;
            }
            HeaterCommand::UpdatePidParameters {
                kp: _,
                ki: _,
                kd: _,
            } => {
                // Ignore for simulation
            }
        };

        // Calculate thermal dynamics
        let power_fraction = current_heater_power as f32 / 10.0;

        // Heat input from heater (degrees per second)
        let heat_input = max_heating_rate * power_fraction;

        // Heat loss to ambient (Newton's law of cooling)
        let temp_diff = current_temp - ambient_temp;
        let mut heat_loss = heat_loss_coefficient * temp_diff;

        // Fan increases heat loss significantly when enabled
        if fan_enabled {
            heat_loss *= 3.0; // Fan triples cooling efficiency
        }

        // Net temperature change considering thermal mass
        let net_heat_rate = (heat_input - heat_loss) * thermal_mass;
        let temp_change = net_heat_rate * time_step;

        // Update temperature
        current_temp += temp_change;

        // Ensure temperature doesn't go below ambient
        if current_temp < ambient_temp {
            current_temp = ambient_temp;
        }

        // Add small amount of realistic noise (±0.1°C)
        let noise = (embassy_time::Instant::now().as_millis() % 200) as f32 / 1000.0 - 0.1;
        let reported_temp = current_temp + noise;

        CURRENT_TEMPERATURE.signal(reported_temp);
        Timer::after_millis(update_interval_ms.into()).await;
    }
}
