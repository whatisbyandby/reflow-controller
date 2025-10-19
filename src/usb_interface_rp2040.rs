use core::cell::RefCell;
use core::str;

use crate::resources_rp2040::USBResources;
use crate::text_interface::{handle_command, to_json_heapless};
use crate::MESSAGE_OUTPUT_CHANNEL;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_usb_logger::ReceiverHandler;
use heapless::String;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

static BUFFER: Mutex<CriticalSectionRawMutex, RefCell<String<2048>>> =
    Mutex::new(RefCell::new(String::new()));

struct Handler;

impl ReceiverHandler for Handler {
    async fn handle_data(&self, data: &[u8]) {
        if let Ok(data) = str::from_utf8(data) {
            BUFFER.lock(|buffer_cell| {
                let mut buffer = buffer_cell.borrow_mut();

                buffer.push_str(data).ok();
                if !data.ends_with('\n') {
                    // Still accumulating
                    return;
                }

                handle_command(&buffer.trim());
                buffer.clear();
            })
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
    spawner.spawn(logger_task(driver).unwrap());

    let receiver = MESSAGE_OUTPUT_CHANNEL.receiver();
    loop {
        let new_message = receiver.receive().await;
        let string_message = to_json_heapless(&new_message);
        log::info!("{}", string_message.as_str());
    }
}
