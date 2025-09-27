use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::rom_data::reset_to_usb_boot;

use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb_logger::ReceiverHandler;
use heapless::String;
use serde::{Serialize, Deserialize};

use crate::{Event, USBResources};
use crate::{ReflowControllerState, CURRENT_STATE, INPUT_EVENT_CHANNEL, PROFILE_LIST_CHANNEL, ACTIVE_PROFILE_CHANNEL};
use crate::profile::Profile;
use core::str;
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// —— USB interrupt binding ——
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

use serde_json_core::ser::to_string;

#[derive(Serialize, Deserialize)]
struct ProfileListResponse {
    profiles: heapless::Vec<heapless::String<64>, 16>,
}

#[derive(Serialize, Deserialize)]
struct ActiveProfileResponse {
    active_profile: Profile,
}

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
            match data {
                "q" => {
                    reset_to_usb_boot(0, 0);
                }
                "START" => {
                    INPUT_EVENT_CHANNEL
                        .sender()
                        .try_send(Event::StartCommand)
                        .unwrap();
                }
                // Add more commands here
                "STOP" => {
                    INPUT_EVENT_CHANNEL
                        .sender()
                        .try_send(Event::StopCommand)
                        .unwrap();
                }
                "RESET" => {
                    INPUT_EVENT_CHANNEL
                        .sender()
                        .try_send(Event::ResetCommand)
                        .unwrap();
                }
                "LIST_PROFILES" => {
                    INPUT_EVENT_CHANNEL
                        .sender()
                        .try_send(Event::ListProfilesRequest)
                        .unwrap();
                }
                _ => {
                    // Check for SET_PROFILE command with parameter
                    if data.starts_with("SET_PROFILE ") {
                        let profile_name = &data[12..]; // Skip "SET_PROFILE "
                        if !profile_name.is_empty() {
                            let mut profile_string = heapless::String::<64>::new();
                            if profile_string.push_str(profile_name).is_ok() {
                                INPUT_EVENT_CHANNEL
                                    .sender()
                                    .try_send(Event::LoadProfile(profile_string))
                                    .unwrap();
                            } else {
                                defmt::warn!("Profile name too long: {}", profile_name);
                            }
                        } else {
                            defmt::warn!("SET_PROFILE command requires a profile name");
                        }
                    } else {
                        defmt::warn!("Unknown command: {}", data);
                    }
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
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver, Handler);
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
        let response = ActiveProfileResponse { active_profile: profile };
        let json: heapless::String<2048> = to_string(&response).unwrap();
        log::info!("{}", json);
    }
}

#[embassy_executor::task]
pub async fn usb_task(spawner: Spawner, r: USBResources) {
    let driver = Driver::new(r.usb, Irqs);
    spawner.spawn(unwrap!(logger_task(driver)));
    spawner.spawn(unwrap!(profile_list_task()));
    spawner.spawn(unwrap!(active_profile_task()));

    let mut receiver = CURRENT_STATE.receiver().unwrap();

    loop {
        let new_state = receiver.get().await;
        let json = to_json_heapless(&new_state);
        log::info!("{}", json);
        Timer::after_secs(1).await;
    }
}
