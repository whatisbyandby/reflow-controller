#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use reflow_controller::{inputs, USBResources};
use reflow_controller::{temperature_sensor::run_temperature_sensor, usb_interface::usb_task};
use {defmt_rtt as _, panic_probe as _};

use reflow_controller::reflow_controller::ReflowController;
use reflow_controller::{
    split_resources, AssignedResources, I2CResources, InputResources, OutputResources,
};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    spawner.spawn(unwrap!(usb_task(spawner, r.usb)));
    // spawner.spawn(unwrap!(display_task(r.display)));
    spawner.spawn(unwrap!(controller_task()));
    spawner.spawn(unwrap!(run_temperature_sensor(r.i2c)));
    spawner.spawn(unwrap!(inputs::interface_task(spawner, r.inputs)));
}

#[embassy_executor::task]
async fn controller_task() {
    let mut controller = ReflowController::new();
    controller.run().await;
}
