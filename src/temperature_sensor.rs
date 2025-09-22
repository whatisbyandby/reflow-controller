use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

use crate::{mcp9600, I2c0Bus};

pub static CURRENT_TEMPERATURE: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn run_temperature_sensor(i2c_bus: &'static I2c0Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut sensor = mcp9600::Mcp9600::new(i2c_dev);

    loop {
        let temp_reading = sensor.read_hot_c().await;
        let temp = match temp_reading {
            Ok(t) => t,
            Err(_) => continue,
        };
        CURRENT_TEMPERATURE.signal(temp);
        Timer::after(Duration::from_millis(500)).await;
    }
}
