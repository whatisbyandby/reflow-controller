#![no_std]

#[cfg(feature = "rp2040")]
pub mod inputs_rp2040;
#[cfg(feature = "rp2040")]
pub use inputs_rp2040 as inputs;

#[cfg(feature = "rp2040")]
pub use defmt as log;

#[cfg(feature = "std")]
pub use log;

#[cfg(feature = "std")]
pub mod inputs_std;
#[cfg(feature = "std")]
pub use inputs_std as inputs;

#[cfg(feature = "rp2040")]
pub mod outputs_rp2040;
#[cfg(feature = "rp2040")]
pub use outputs_rp2040 as outputs;

#[cfg(feature = "std")]
pub mod outputs_std;
#[cfg(feature = "std")]
pub use outputs_std as outputs;

pub mod pid;
pub mod profile;
pub mod reflow_controller;
pub mod sd_profile_reader;

#[cfg(feature = "rp2040")]
pub mod resources_rp2040;
#[cfg(feature = "rp2040")]
pub use resources_rp2040 as resources;

#[cfg(feature = "rp2040")]
pub mod heater_rp2040;
#[cfg(feature = "rp2040")]
pub use heater_rp2040 as heater;

#[cfg(feature = "std")]
pub mod heater_std;
#[cfg(feature = "std")]
pub use heater_std as heater;

#[cfg(feature = "rp2040")]
pub mod temperature_sensor_mcp9600;
#[cfg(feature = "rp2040")]
pub use temperature_sensor_mcp9600 as temperature_sensor;

#[cfg(feature = "std")]
pub mod temperature_sensor_mock;
#[cfg(feature = "std")]
pub use temperature_sensor_mock as temperature_sensor;

#[cfg(feature = "rp2040")]
pub mod usb_interface_rp2040;
#[cfg(feature = "rp2040")]
pub use usb_interface_rp2040 as usb_interface;

#[cfg(feature = "std")]
pub mod usb_interface_std;
#[cfg(feature = "std")]
pub use usb_interface_std as usb_interface;

pub static VERSION: &str = "v0.1";
pub static SYSTEM_TICK_MILLIS: u32 = 100;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    StartCommand,
    StopCommand,
    ResetCommand,
    DoorStateChanged(bool),            // true = closed, false = opened
    LoadProfile(heapless::String<64>), // filename to load from SD card
    ListProfilesRequest,
    SimulationReset,
    UpdatePidParameters { kp: f32, ki: f32, kd: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    LedOn,
    LedOff,
    Blink(u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCommand {
    SetFan(bool),
    SetLight(bool),
    SetBuzzer(bool),
    SetStartButtonLight(LedState),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeaterCommand {
    SetPower(u8),
    SetFan(bool),
    SimulationReset,
    UpdatePidParameters { kp: f32, ki: f32, kd: f32 },
}

pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Event, 3> = Channel::new();
pub static OUTPUT_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, OutputCommand, 3> =
    Channel::new();
pub static HEATER_POWER: Channel<CriticalSectionRawMutex, HeaterCommand, 2> = Channel::new();
pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 3> = Watch::new();
pub static PROFILE_LIST_CHANNEL: Channel<
    CriticalSectionRawMutex,
    heapless::Vec<heapless::String<64>, 16>,
    1,
> = Channel::new();
pub static ACTIVE_PROFILE_CHANNEL: Channel<CriticalSectionRawMutex, profile::Profile, 1> =
    Channel::new();

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Initializing,
    Idle,
    Running,
    Finished,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflowControllerState {
    pub status: Status,
    pub target_temperature: f32,
    pub current_temperature: f32,
    pub door_closed: bool,
    pub fan: bool,
    pub light: bool,
    pub heater_power: u8, // value between 0 and 100
    pub timer: u32,
    pub current_step: &'static str,
    pub current_profile: heapless::String<32>,
    pub error_message: heapless::String<256>,
}
