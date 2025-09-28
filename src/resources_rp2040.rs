use assign_resources::assign_resources;
pub use embassy_rp::i2c;
use embassy_rp::i2c::I2c;
use embassy_rp::peripherals;
use embassy_rp::peripherals::I2C0;
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

pub type I2c0Bus = Mutex<NoopRawMutex, I2c<'static, I2C0, i2c::Async>>;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

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
