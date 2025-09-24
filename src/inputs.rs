use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Input, Level, Pull},
    peripherals::{PIN_12, PIN_13, PIN_14, PIN_15, PIN_4, PIN_5},
    Peri,
};
use embassy_time::Timer;

use crate::{Event, InputResources, INPUT_EVENT_CHANNEL};

#[embassy_executor::task]
pub async fn interface_task(spawner: Spawner, r: InputResources) {
    spawner.spawn(unwrap!(button_a_task(r.button_a)));
    spawner.spawn(unwrap!(button_b_task(r.button_b)));
    spawner.spawn(unwrap!(button_x_task(r.button_x)));
    spawner.spawn(unwrap!(button_y_task(r.button_y)));
    spawner.spawn(unwrap!(door_switch_task(r.door_switch)));
    spawner.spawn(unwrap!(start_button_task(r.start_button)));
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

        Timer::after_millis(100).await; // Debounce delay
    }
}

#[embassy_executor::task]
async fn button_y_task(pin: Peri<'static, PIN_15>) -> ! {
    let mut button = Input::new(pin, Pull::Up);
    loop {
        button.wait_for_falling_edge().await;
        let sender = INPUT_EVENT_CHANNEL.sender();
        defmt::info!("Button Y Pressed");
        sender.send(Event::ResetCommand).await;
        Timer::after_millis(100).await; // Debounce delay
    }
}

#[embassy_executor::task]
async fn start_button_task(pin: Peri<'static, PIN_5>) -> ! {
    let mut button = Input::new(pin, Pull::Up);
    loop {
        button.wait_for_falling_edge().await;
        defmt::info!("Start Button Pressed");
        INPUT_EVENT_CHANNEL.sender().send(Event::StartCommand).await;
        Timer::after_millis(100).await; // Debounce delay
    }
}

#[embassy_executor::task]
async fn door_switch_task(pin: Peri<'static, PIN_4>) -> ! {
    let mut door_switch = Input::new(pin, Pull::Up);
    {
        let current_state = door_switch.get_level();

        let sender = INPUT_EVENT_CHANNEL.sender();
        match current_state {
            Level::Low => {
                sender.send(Event::DoorStateChanged(true)).await;
            }
            Level::High => {
                sender.send(Event::DoorStateChanged(false)).await;
            }
        }
    }

    loop {
        // Wait for a change in the door switch state
        door_switch.wait_for_any_edge().await;
        defmt::info!("Door switch state changed");
        Timer::after_millis(500).await; // Debounce delay

        let new_state = door_switch.get_level();

        let sender = INPUT_EVENT_CHANNEL.sender();
        match new_state {
            Level::Low => {
                sender.send(Event::DoorStateChanged(true)).await;
            }
            Level::High => {
                sender.send(Event::DoorStateChanged(false)).await;
            }
        }
    }
}
