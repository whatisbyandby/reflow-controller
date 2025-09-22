#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::I2C0;
use embassy_sync::mutex::Mutex;
use reflow_controller::heater::heater_task;

use reflow_controller::inputs::interface_task;
use reflow_controller::outputs::output_task;
use reflow_controller::{temperature_sensor::run_temperature_sensor, usb_interface::usb_task};
use reflow_controller::{I2c0Bus, USBResources};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use reflow_controller::reflow_controller::controller_task;
use reflow_controller::{
    split_resources, AssignedResources, I2CResources, InputResources, OutputResources,
};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    bind_interrupts!(struct Irqs {
        I2C0_IRQ => InterruptHandler<I2C0>;
    });

    // Shared I2C bus
    let i2c = I2c::new_async(r.i2c.i2c, r.i2c.scl, r.i2c.sda, Irqs, Config::default());
    static I2C_BUS: StaticCell<I2c0Bus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    spawner.spawn(unwrap!(usb_task(spawner, r.usb)));
    spawner.spawn(unwrap!(heater_task(i2c_bus)));
    spawner.spawn(unwrap!(controller_task()));
    spawner.spawn(unwrap!(run_temperature_sensor(i2c_bus)));
    spawner.spawn(unwrap!(interface_task(spawner, r.inputs)));
    spawner.spawn(unwrap!(output_task(spawner, r.outputs)));
}
