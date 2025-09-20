use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::rom_data::reset_to_usb_boot;

use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb_logger::ReceiverHandler;
use heapless::String;

use core::str;
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use crate::reflow_controller::{Command, ReflowControllerState, CURRENT_STATE};
use crate::USBResources;

// —— USB interrupt binding ——
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

use serde_json_core::{de::from_str, ser::to_string};

pub fn to_json_heapless(msg: &ReflowControllerState) -> String<1024> {
    // Writes JSON into your buffer; returns (&str, usize)
    let out = to_string(msg).unwrap();
    out
}

struct Handler;

impl ReceiverHandler for Handler {
    async fn handle_data(&self, data: &[u8]) {
        if let Ok(data) = str::from_utf8(data) {
            let data = data.trim();

            // If you are using elf2uf2-term with the '-t' flag, then when closing the serial monitor,
            // this will automatically put the pico into boot mode
            if data == "q" || data == "elf2uf2-term" {
                reset_to_usb_boot(0, 0); // Restart the chip
            } else if let Ok(cmd) = from_str::<Command>(data) {
                // Handle other commands here
                defmt::info!("Received command: {:?}", cmd);
                // You can add code to process the command as needed
            } else {
                defmt::warn!("Unknown command: {}", data);
            }
        }
    }

    fn new() -> Self {
        Self
    }
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver, Handler);
}

#[embassy_executor::task]
pub async fn usb_task(spawner: Spawner, r: USBResources) {
    let driver = Driver::new(r.usb, Irqs);
    spawner.spawn(logger_task(driver)).unwrap();

    let mut receiver = CURRENT_STATE.receiver().unwrap();

    loop {
        let new_state = receiver.get().await;
        let json = to_json_heapless(&new_state);
        log::info!("{}", json);
        Timer::after_secs(1).await;
    }
}
