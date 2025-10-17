use crate::{
    resources_rp2040::OutputResources, LedState, OutputCommand, OUTPUT_COMMAND_CHANNEL,
    SYSTEM_TICK_MILLIS,
};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use embassy_time::Timer;

pub static LED_STATE: Watch<CriticalSectionRawMutex, LedState, 1> = Watch::new();

#[embassy_executor::task]
pub async fn output_task(spawner: Spawner, r: OutputResources) {
    Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;

    let start_button_light = Output::new(r.start_button_light, Level::Low);

    let receiver = OUTPUT_COMMAND_CHANNEL.receiver();
    spawner.spawn(unwrap!(start_button_light_task(start_button_light)));

    loop {
        let command = receiver.receive().await;
        match command {
            OutputCommand::SetStartButtonLight(state) => LED_STATE.sender().send(state),
        }
    }
}

#[embassy_executor::task]
pub async fn start_button_light_task(mut start_button_light: Output<'static>) {
    let mut receiver = LED_STATE.receiver().unwrap();

    loop {
        let state = receiver.changed().await;

        match state {
            LedState::LedOn => start_button_light.set_level(Level::High),
            LedState::LedOff => start_button_light.set_level(Level::Low),
            LedState::Blink(on_duration, off_duration) => 'blink: loop {
                if receiver.try_changed().is_some() {
                    break 'blink;
                }
                start_button_light.set_level(Level::High);
                Timer::after_millis(on_duration.into()).await;
                start_button_light.set_level(Level::Low);
                Timer::after_millis(off_duration.into()).await;
            },
        }
    }
}
