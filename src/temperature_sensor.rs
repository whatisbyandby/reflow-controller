use defmt::{debug, info};
use embassy_rp::i2c::InterruptHandler;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

use crate::{mcp9600, I2CResources};

pub static CURRENT_TEMPERATURE: Signal<CriticalSectionRawMutex, f32> = Signal::new();

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<embassy_rp::peripherals::I2C0>;
});

#[embassy_executor::task]
pub async fn run_temperature_sensor(r: I2CResources) {
    let config = embassy_rp::i2c::Config::default();
    let mut bus = embassy_rp::i2c::I2c::new_async(r.i2c, r.scl, r.sda, Irqs, config);

    let mcp9600 = mcp9600::Mcp9600::new(0x67);
    mcp9600.init(&mut bus).await.unwrap();

    loop {
        let temp = mcp9600.read_hot_c(&mut bus).await.unwrap();
        CURRENT_TEMPERATURE.signal(temp);
        Timer::after(Duration::from_millis(500)).await;
    }
}
