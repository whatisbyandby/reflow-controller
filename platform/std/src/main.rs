use embassy_executor::Spawner;
use embassy_time::Timer;
use log::*;
use reflow_controller::inputs::interface_task;
use reflow_controller::outputs::output_task;
use reflow_controller::reflow_controller::controller_task;
use reflow_controller::{
    heater::heater_task, temperature_sensor::run_temperature_sensor, usb_interface::usb_task,
};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp_nanos()
        .init();

    spawner.spawn(heater_task().unwrap());
    spawner.spawn(run_temperature_sensor().unwrap());

    spawner.spawn(interface_task(spawner).unwrap());
    spawner.spawn(output_task(spawner).unwrap());

    spawner.spawn(usb_task(spawner).unwrap());
    spawner.spawn(controller_task().unwrap());
}
