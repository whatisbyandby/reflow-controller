use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::with_timeout;
use embassy_time::{Duration, Timer};

use crate::{mcp9600, I2c0Bus};

pub static CURRENT_TEMPERATURE: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[cfg(not(feature = "mock_temperature_sensor"))]
#[embassy_executor::task]
pub async fn run_temperature_sensor(i2c_bus: &'static I2c0Bus) -> ! {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut sensor = mcp9600::Mcp9600::new(i2c_dev);

    info!("Starting temperature sensor task");

    loop {
        let temp_reading = with_timeout(Duration::from_millis(200), sensor.read_hot_c()).await;
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
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[cfg(feature = "mock_temperature_sensor")]
#[embassy_executor::task]
pub async fn run_temperature_sensor(i2c_bus: &'static I2c0Bus) -> ! {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut sensor = mcp9600::Mcp9600::new(i2c_dev);

    loop {
        let temp_reading = with_timeout(Duration::from_millis(200), sensor.read_hot_c()).await;
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
        Timer::after(Duration::from_millis(500)).await;
    }
}
