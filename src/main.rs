#![no_std]
#![no_main]


use defmt::*;
use embassy_executor::Spawner;
use reflow_controller::usb_interface::usb_task;
use reflow_controller::USBResources;
use {defmt_rtt as _, panic_probe as _};

use reflow_controller::reflow_controller::{ReflowController, CURRENT_STATE};
use reflow_controller::{split_resources, AssignedResources, DisplayResources};


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    spawner.spawn(unwrap!(usb_task(spawner, r.usb)));
    // spawner.spawn(unwrap!(display_task(r.display)));
    spawner.spawn(unwrap!(controller_task()));
    // spawner.spawn(unwrap!(run_temperature_sensor()));
}

#[embassy_executor::task]
async fn controller_task() {
    let mut controller = ReflowController::new();
    controller.run().await;
}

// #[embassy_executor::task]
// async fn display_task(r: DisplayResources) {
//     // create SPI
//     let mut display_config = spi::Config::default();
//     display_config.phase = spi::Phase::CaptureOnSecondTransition;
//     display_config.polarity = spi::Polarity::IdleHigh;

//     let spi = Spi::new_blocking(r.spi, r.clk, r.mosi, r.miso, display_config.clone());
//     let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));
//     let display_spi =
//         SpiDeviceWithConfig::new(&spi_bus, Output::new(r.cs, Level::High), display_config);

//     let dcx = Output::new(r.dcx, Level::Low);
//     let di = SPIInterface::new(display_spi, dcx);

//     // Define the display from the display interface and initialize it
//     let mut display = Builder::new(ST7789, di)
//         .display_size(240, 240)
//         .orientation(Orientation::new().rotate(Rotation::Deg0))
//         .init(&mut Delay)
//         .unwrap();

//     let mut receiver = CURRENT_STATE.receiver().unwrap();

//     loop {
//         let new_state = receiver.changed().await;
//         draw_page(&mut display, &new_state);
//     }
// }
