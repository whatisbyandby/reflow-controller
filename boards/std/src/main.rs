use embassy_executor::Spawner;
use embassy_time::Timer;
use log::*;
use reflow_controller::reflow_controller::ReflowController;

#[embassy_executor::task]
async fn run(spawner: Spawner) {
    let mut controller = ReflowController::new();
    controller.run(spawner).await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp_nanos()
        .init();

    spawner.spawn(run(spawner)).unwrap();
}