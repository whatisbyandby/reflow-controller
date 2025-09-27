#![no_std]

pub mod heater;
pub mod inputs;
pub mod mcp9600;
pub mod outputs;
pub mod pid;
pub mod profile;
pub mod reflow_controller;
pub mod relay;
pub mod sd_profile_reader;
use defmt::Format;

pub mod temperature_sensor;
pub mod usb_interface;
pub static VERSION: &str = "v0.1";

use assign_resources::assign_resources;
use embassy_rp::i2c::I2c;
use embassy_rp::i2c::{self};
use embassy_rp::peripherals;
use embassy_rp::peripherals::I2C0;
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use serde::{Deserialize, Serialize};

pub type I2c0Bus = Mutex<NoopRawMutex, I2c<'static, I2C0, i2c::Async>>;

pub async fn scan_i2c_bus<I2C, E>(i2c: &mut I2C) -> heapless::Vec<u8, 128>
where
    I2C: embedded_hal_async::i2c::I2c<Error = E>,
{
    let mut devices = heapless::Vec::new();

    for addr in 0x08..=0x77 {
        let result = i2c.write(addr, &[]).await;
        if result.is_ok() {
            let _ = devices.push(addr);
        }
    }

    devices
}

pub async fn scan_shared_i2c_bus(i2c_bus: &I2c0Bus) -> heapless::Vec<u8, 128> {
    let mut i2c = i2c_bus.lock().await;
    scan_i2c_bus(&mut *i2c).await
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    StartCommand,
    StopCommand,
    ResetCommand,
    DoorStateChanged(bool), // true = closed, false = opened
    LoadProfile(heapless::String<64>), // filename to load from SD card
    ListProfilesRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum LedState {
    LedOn,
    LedOff,
    Blink(u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum OutputCommand {
    SetFan(bool),
    SetLight(bool),
    SetBuzzer(bool),
    SetStartButtonLight(LedState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum HeaterCommand {
    SetPower(u8),
    SetFan(bool),
}

pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Event, 3> = Channel::new();
pub static OUTPUT_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, OutputCommand, 3> =
    Channel::new();
pub static HEATER_POWER: Channel<CriticalSectionRawMutex, HeaterCommand, 2> = Channel::new();
pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 3> = Watch::new();
pub static PROFILE_LIST_CHANNEL: Channel<CriticalSectionRawMutex, heapless::Vec<heapless::String<64>, 16>, 1> = Channel::new();
pub static ACTIVE_PROFILE_CHANNEL: Channel<CriticalSectionRawMutex, profile::Profile, 1> = Channel::new();

#[derive(Debug, Clone, PartialEq, Format, Serialize, Deserialize)]
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

assign_resources! {
    inputs: InputResources {
        button_a: PIN_12,
        button_b: PIN_13,
        button_x: PIN_14,
        button_y: PIN_15,
        door_switch: PIN_4,
        start_button: PIN_5,
    },
    outputs: OutputResources {
        fan: PIN_17,
        light: PIN_18,
        buzzer: PIN_19,
        start_button_light: PIN_3,
    },
    usb: USBResources {
        usb: USB,
    },
    i2c: I2CResources {
        i2c: I2C0,
        sda: PIN_20,
        scl: PIN_21,
    },
    // SD card resources - will be added when hardware integration is ready
    // sd_card: SdCardResources {
    //     spi: SPI0,
    //     miso: PIN_16,
    //     mosi: PIN_19,
    //     clk: PIN_18,
    //     cs: PIN_17,
    // },
}
