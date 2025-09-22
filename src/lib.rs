#![no_std]

pub mod heater;
pub mod inputs;
pub mod outputs;
pub mod mcp9600;
pub mod pid;
pub mod profile;
pub mod reflow_controller;
pub mod relay;
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Format)]
pub enum InputEvent {
    ButtonAPressed,
    ButtonBPressed,
    ButtonXPressed,
    ButtonYPressed,
    StartButtonPressed,
    DoorOpened,
    DoorClosed,
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

pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, InputEvent, 3> = Channel::new();
pub static OUTPUT_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, OutputCommand, 3> = Channel::new();
pub static HEATER_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, u8, 3> = Channel::new();
pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 3> = Watch::new();

#[derive(Debug, Clone, PartialEq, Format, Serialize, Deserialize)]
pub enum Status {
    Initializing,
    Idle,
    Running,
    Error,
}

#[derive(Debug, Clone, Format, Serialize, Deserialize)]
pub struct ReflowControllerState {
    pub status: Status,
    pub target_temperature: f32,
    pub current_temperature: f32,
    pub door_closed: bool,
    pub fan: bool,
    pub light: bool,
    pub heater_power: u8, // value between 0 and 100
    pub total_time_remaining: u32,
    pub step_time_remaining: u32,
    pub current_step: &'static str,
    pub current_profile: &'static str,
}

assign_resources! {
    inputs: InputResources {
        button_a: PIN_12,
        button_b: PIN_13,
        button_x: PIN_14,
        button_y: PIN_15,
        door_switch: PIN_4,
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
}
