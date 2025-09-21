use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Input, Pull},
    peripherals::{PIN_12, PIN_13, PIN_14, PIN_22, PIN_26, PIN_27},
    Peri,
};
use embassy_time::Timer;

use crate::InputResources;

#[embassy_executor::task]
pub async fn interface_task(spawner: Spawner, r: InputResources) {
    spawner.spawn(unwrap!(button_a_task(r.button_a)));
    spawner.spawn(unwrap!(button_b_task(r.button_b)));
    spawner.spawn(unwrap!(button_x_task(r.button_x)));
}

#[embassy_executor::task]
async fn button_a_task(pin: Peri<'static, PIN_12>) -> ! {
    let mut button = Input::new(pin, Pull::Up);
    loop {
        button.wait_for_falling_edge().await;
        defmt::info!("Button A Pressed");
        // Handle button one press
        Timer::after_millis(100).await; // Debounce delay
    }
}

#[embassy_executor::task]
async fn button_b_task(pin: Peri<'static, PIN_13>) -> ! {
    let mut button = Input::new(pin, Pull::Up);
    loop {
        button.wait_for_falling_edge().await;
        defmt::info!("Button B Pressed");
        // Handle button one press
        Timer::after_millis(100).await; // Debounce delay
    }
}

#[embassy_executor::task]
async fn button_x_task(pin: Peri<'static, PIN_14>) -> ! {
    let mut button = Input::new(pin, Pull::Up);
    loop {
        button.wait_for_falling_edge().await;
        defmt::info!("Button X Pressed");
        // Handle button one press
        Timer::after_millis(100).await; // Debounce delay
    }
}
