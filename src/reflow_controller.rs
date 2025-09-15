use crate::display::{Events, EVENT_CHANNEL};
use crate::profile::{Step, PROFILES};
use crate::temperature_sensor::{run_temperature_sensor, CURRENT_TEMPERATURE};
use defmt::{info, Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Instant, Timer};
use serde::{Deserialize, Serialize};
use {defmt_rtt as _, panic_probe as _};

pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 3> = Watch::new();

#[derive(Debug, PartialEq, Serialize, Deserialize, Format)]
pub enum Command {
    SetTemperature(f32), // set Temperature Command
    Fan(bool),           // Fan On/Off Command
    Light(bool),         // Light On/Off Command
    Off,                 // Turn Off Command
}

#[derive(Debug, Clone, PartialEq, Format, Serialize, Deserialize)]
pub enum Status {
    Initializing,
    Idle,
    Running,
    Error,
}

pub struct ReflowController {
    target_temperature: f32,
    current_temperature: f32,
    door_closed: bool,
    fan: bool,
    light: bool,
    heater_power: u32, // value between 0 and 100
    profile_index: usize,
    current_step_index: usize,
    status: Status,
    current_step_start_time: Instant,
    profile_start_time: Instant,
    step_time_remaining: u32,
}

#[derive(Debug, Clone, Format, Serialize, Deserialize)]
pub struct ReflowControllerState {
    pub status: Status,
    pub target_temperature: f32,
    pub current_temperature: f32,
    pub door_closed: bool,
    pub fan: bool,
    pub light: bool,
    pub heater_power: u32, // value between 0 and 100
    pub total_time_remaining: u32,
    pub step_time_remaining: u32,
    pub current_step: u8,
    pub current_profile: u8,
}

impl ReflowController {
    pub fn new() -> Self {
        Self {
            target_temperature: 0.0,
            current_temperature: 25.0,
            door_closed: false,
            fan: false,
            light: false,
            heater_power: 0,
            profile_index: 0,
            current_step_index: 0,
            status: Status::Initializing,
            current_step_start_time: Instant::now(),
            profile_start_time: Instant::now(),
            step_time_remaining: 0,
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.status {
                Status::Initializing => {
                    self.init().await;
                }
                Status::Idle => self.idle().await,
                Status::Running => self.running().await,
                Status::Error => self.error().await,
            }
            self.send_state();
            Timer::after(Duration::from_millis(1000)).await;
        }
    }

    async fn init(&mut self) {
        Timer::after_secs(1).await;
        self.status = Status::Idle;
    }

    async fn idle(&mut self) {
        Timer::after_secs(1).await;
    }

    async fn running(&mut self) {
        Timer::after_secs(1).await;
    }

    async fn error(&mut self) {
        Timer::after_secs(1).await;
    }

    fn send_state(&mut self) {
        let snd = CURRENT_STATE.sender();
        let state = ReflowControllerState {
            status: self.status.clone(),
            target_temperature: self.target_temperature,
            current_temperature: self.current_temperature,
            door_closed: self.door_closed,
            fan: self.fan,
            light: self.light,
            heater_power: self.heater_power,
            total_time_remaining: 0,
            step_time_remaining: self.step_time_remaining,
            current_profile: self.profile_index as u8,
            current_step: self.current_step_index as u8,
        };
        snd.send(state);
    }

    async fn handle_new_temperature(&mut self, new_temperature: f32) {
        self.current_temperature = new_temperature;
    }
}
