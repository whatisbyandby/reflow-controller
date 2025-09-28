#[path = "./mcp9600.rs"]
mod mcp9600;

use crate::log::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::with_timeout;

use crate::resources_rp2040::I2c0Bus;
use crate::SYSTEM_TICK_MILLIS;

pub static CURRENT_TEMPERATURE: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn run_temperature_sensor(i2c_bus: &'static I2c0Bus) -> ! {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut sensor = mcp9600::Mcp9600::new(i2c_dev);

    info!("Starting temperature sensor task");

    loop {
        let temp_reading = with_timeout(
            Duration::from_millis((SYSTEM_TICK_MILLIS * 2).into()),
            sensor.read_hot_c(),
        )
        .await;
        let temp = match temp_reading {
            Ok(Ok(t)) => t,
            Ok(Err(_)) => {
                error!("Error reading temperature");
                continue;
            }
            Err(_) => {
                error!("Temperature read timed out");
                continue;
            }
        };
        CURRENT_TEMPERATURE.signal(temp);
        Timer::after_millis((SYSTEM_TICK_MILLIS * 5).into()).await;
    }
}
