use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::rom_data::reset_to_usb_boot;

use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb_logger::ReceiverHandler;

use crate::resources_rp2040::USBResources;
use crate::text_interface::{
    handle_command, to_json_heapless, ActiveProfileResponse, ProfileListResponse,
};
use crate::{
    SdResponse, ACTIVE_PROFILE_CHANNEL, CURRENT_STATE, PROFILE_LIST_CHANNEL, SD_RESPONSE_CHANNEL,
    SYSTEM_TICK_MILLIS,
};
use core::str;
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use heapless::Vec;
use serde_json_core::ser::to_string;

// —— USB interrupt binding ——
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// Static line buffer for accumulating USB data until newline
static LINE_BUFFER: Mutex<CriticalSectionRawMutex, Vec<u8, 4096>> = Mutex::new(Vec::new());

struct Handler;

impl ReceiverHandler for Handler {
    async fn handle_data(&self, data: &[u8]) {
        let mut buffer = LINE_BUFFER.lock().await;

        // Process each byte, looking for newlines
        for &byte in data {
            if byte == b'\n' || byte == b'\r' {
                // End of line - process the command if buffer has data
                if !buffer.is_empty() {
                    // Handle platform-specific command first (q = reset to USB bootloader)
                    if let Ok(text) = str::from_utf8(&buffer) {
                        if text.trim() == "q" {
                            reset_to_usb_boot(0, 0);
                        }
                    }

                    // Handle common commands
                    handle_command(&buffer);
                    buffer.clear();
                }
            } else {
                // Add byte to buffer if there's space
                if buffer.push(byte).is_err() {
                    defmt::warn!("Command too long, discarding buffer");
                    buffer.clear();
                }
            }
        }
    }

    fn new() -> Self {
        Self
    }
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(3072, log::LevelFilter::Info, driver, Handler);
}

#[embassy_executor::task]
async fn profile_list_task() {
    let receiver = PROFILE_LIST_CHANNEL.receiver();
    loop {
        let profiles = receiver.receive().await;
        let response = ProfileListResponse { profiles };
        let json: heapless::String<1024> = to_string(&response).unwrap();
        log::info!("{}", json);
    }
}

#[embassy_executor::task]
async fn active_profile_task() {
    let receiver = ACTIVE_PROFILE_CHANNEL.receiver();
    loop {
        let profile = receiver.receive().await;
        let response = ActiveProfileResponse {
            active_profile: profile,
        };
        let json: heapless::String<2048> = to_string(&response).unwrap();
        log::info!("{}", json);
    }
}

#[embassy_executor::task]
async fn sd_response_task() {
    let receiver = SD_RESPONSE_CHANNEL.receiver();
    loop {
        let response = receiver.receive().await;
        match response {
            SdResponse::ProfileList(profiles) => {
                let response = ProfileListResponse { profiles };
                let json: heapless::String<1024> = to_string(&response).unwrap();
                log::info!("{}", json);
            }
            SdResponse::ProfileData(profile) => {
                // Forward profile to controller
                ACTIVE_PROFILE_CHANNEL.send(profile.clone()).await;

                // Also send as response
                let response = ActiveProfileResponse {
                    active_profile: profile,
                };
                let json: heapless::String<2048> = to_string(&response).unwrap();
                log::info!("{}", json);
            }
            SdResponse::WriteSuccess => {
                log::info!("{{\"write_success\": true}}");
            }
            SdResponse::Error(err) => {
                log::error!("SD Card Error: {:?}", err);
            }
        }
    }
}

#[embassy_executor::task]
pub async fn usb_task(spawner: Spawner, r: USBResources) {
    let driver = Driver::new(r.usb, Irqs);
    spawner.spawn(unwrap!(logger_task(driver)));
    spawner.spawn(unwrap!(profile_list_task()));
    spawner.spawn(unwrap!(active_profile_task()));
    spawner.spawn(unwrap!(sd_response_task()));

    let mut receiver = CURRENT_STATE.receiver().unwrap();

    loop {
        let new_state = receiver.changed().await;
        let json = to_json_heapless(&new_state);
        log::info!("{}", json);
    }
}
