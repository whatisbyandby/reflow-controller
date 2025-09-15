use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

pub static CURRENT_TEMPERATURE: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn run_temperature_sensor() {
    let mut current_temp = 25.0;

    loop {
        Timer::after(Duration::from_millis(500)).await;

        current_temp += 0.1;

        CURRENT_TEMPERATURE.signal(current_temp);
    }
}
