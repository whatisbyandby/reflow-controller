#![no_std]

pub mod display;
mod home_screen;
pub mod pid;
pub mod profile;
pub mod reflow_controller;
mod running_screen;
mod splash_screen;
pub mod temperature_sensor;
pub mod usb_interface;

pub static VERSION: &str = "v0.1";

use assign_resources::assign_resources;
use embassy_rp::peripherals;
use embassy_rp::Peri;

assign_resources! {
    display: DisplayResources {
        spi: SPI0,
        miso: PIN_20,
        mosi: PIN_19,
        cs: PIN_17,
        clk: PIN_18,
        dcx: PIN_16,
    },
    usb: USBResources {
        usb: USB,
    },
}
