#![no_std]

pub mod pid;
pub mod profile;
pub mod reflow_controller;

pub mod text_interface;

#[cfg(feature = "rp2040")]
pub use defmt as log;

#[cfg(feature = "rp2040")]
pub mod inputs_rp2040;
#[cfg(feature = "rp2040")]
pub use inputs_rp2040 as inputs;

#[cfg(feature = "rp2040")]
pub mod outputs_rp2040;

#[cfg(feature = "rp2040")]
pub use outputs_rp2040 as outputs;

#[cfg(feature = "rp2040")]
pub mod resources_rp2040;
#[cfg(feature = "rp2040")]
pub use resources_rp2040 as resources;

#[cfg(feature = "rp2040")]
pub mod heater_rp2040;
#[cfg(feature = "rp2040")]
pub use heater_rp2040 as heater;

pub mod temperature_sensor_mcp9600;
pub use temperature_sensor_mcp9600 as temperature_sensor;

#[cfg(feature = "rp2040")]
pub mod usb_interface_rp2040;
#[cfg(feature = "rp2040")]
pub use usb_interface_rp2040 as usb_interface;

#[cfg(feature = "rp2040")]
pub mod sd_card_rp2040;
#[cfg(feature = "rp2040")]
pub use sd_card_rp2040 as sd_profile_reader;

pub static VERSION: &str = "v0.1";
pub static SYSTEM_TICK_MILLIS: u32 = 100;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;
use heapless::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    StartCommand,
    StopCommand,
    ResetCommand,
    DoorStateChanged(bool),
    LoadProfile(String<14>),   // filename to load from SD card
    RemoveProfile(String<14>), // filename to remove from SD card
    ListProfilesRequest,
    SimulationReset,
    UpdatePidParameters {
        kp: f32,
        ki: f32,
        kd: f32,
    },
    FanControl(bool), // true = on, false = off
    StartPidTuning,
    WriteProfile {
        filename: String<14>,
        profile_json: String<2048>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    LedOn,
    LedOff,
    Blink(u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCommand {
    SetStartButtonLight(LedState),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeaterCommand {
    SetPower(u8),
    SetFan(bool),
    SimulationReset,
    UpdatePidParameters { kp: f32, ki: f32, kd: f32 },
}

#[derive(Debug, Clone)]
pub enum SdCommand {
    ListProfiles,
    DeleteProfile {
        filename: String<14>,
    },
    ReadProfile {
        filename: String<14>,
    },
    WriteProfile {
        filename: String<14>,
        profile: profile::Profile,
    },
}

#[derive(Debug, Clone)]
pub enum SdResponse {
    ProfileList(heapless::Vec<String<14>, 16>),
    ProfileData(profile::Profile),
    WriteSuccess,
    DeleteSuccess,
    Error(SdProfileError),
}

#[derive(Debug, Clone)]
pub enum SdProfileError {
    SdCardError,
    FileNotFound,
    ParseError,
    InvalidFormat,
    TooManyProfiles,
    Unknown,
}

pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Event, 3> = Channel::new();
pub static MESSAGE_OUTPUT_CHANNEL: Channel<CriticalSectionRawMutex, ReflowControllerMessage, 3> =
    Channel::new();
pub static OUTPUT_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, OutputCommand, 3> =
    Channel::new();
pub static HEATER_POWER: Channel<CriticalSectionRawMutex, HeaterCommand, 2> = Channel::new();
pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 3> = Watch::new();

pub static SD_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, SdCommand, 2> = Channel::new();
pub static SD_RESPONSE_CHANNEL: Channel<CriticalSectionRawMutex, SdResponse, 2> = Channel::new();

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Initializing,
    Idle,
    Running,
    Finished,
    Error,
    PidTuning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteProfileResult {
    pub success: bool,
    pub filename: heapless::String<64>,
    pub message: heapless::String<128>,
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
    pub timer: u32,       // this is in ms
    pub current_step: heapless::String<32>,
    pub current_profile: heapless::String<32>,
    pub error_message: heapless::String<256>,
    pub pid_kp: f32,
    pub pid_ki: f32,
    pub pid_kd: f32,
    pub pid_p_term: f32,
    pub pid_i_term: f32,
    pub pid_d_term: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReflowControllerMessage {
    StateUpdate(ReflowControllerState),
    ProfileList(heapless::Vec<heapless::String<14>, 16>),
    ActiveProfile(profile::Profile),
    WriteProfileResult(WriteProfileResult),
}
