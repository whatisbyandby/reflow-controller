#![no_std]

pub mod inputs;
pub mod mcp9600;
pub mod pid;
pub mod profile;
pub mod reflow_controller;
use defmt::Format;

pub mod temperature_sensor;
pub mod usb_interface;
pub static VERSION: &str = "v0.1";

use assign_resources::assign_resources;
use embassy_rp::peripherals;
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Format)]
pub enum InputEvent {
    ButtonAPressed,
    ButtonBPressed,
    ButtonXPressed,
    ButtonYPressed,
    DoorOpened,
    DoorClosed,
}

pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, InputEvent, 3> = Channel::new();

assign_resources! {
    inputs: InputResources {
        button_a: PIN_12,
        button_b: PIN_13,
        button_x: PIN_14,
        button_y: PIN_15,
    },
    outputs: OutputResources {
        fan: PIN_17,
        light: PIN_18,
        buzzer: PIN_19,
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
