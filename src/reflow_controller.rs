use crate::display::{Events, EVENT_CHANNEL};
use crate::profile::PROFILES;
use crate::temperature_sensor::{run_temperature_sensor, CURRENT_TEMPERATURE};
use crate::{display::display_task};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;

pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 2> = Watch::new();

#[derive(Debug, PartialEq)]
pub enum Command {
    SetTemperature(f32), // set Temperature Command
    Fan(bool),           // Fan On/Off Command
    Light(bool),         // Light On/Off Command
    Off,                 // Turn Off Command
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Initializing(u8),
    Idle,
    Running,
    Error(String<32>),
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
    step_time_remaining: u32,
}

#[derive(Debug, Clone)]
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
            status: Status::Initializing(0),
            current_step_start_time: Instant::now(),
            step_time_remaining: 0,
        }
    }

    pub async fn run(&mut self, spawner: Spawner) {
        self.send_state().await;

        spawner.spawn(run_temperature_sensor()).unwrap();
        spawner.spawn(display_task()).unwrap();

        loop {
            match self.status {
                Status::Initializing(percent) => self.handle_init(percent).await,
                Status::Idle => {
                    self.handle_idle_state().await;
                }
                Status::Running => {
                    self.handle_running_state().await;
                }
                Status::Error(ref msg) => {
                    
                }
            }
            Timer::after(Duration::from_millis(40)).await;
        }
    }

    async fn send_state(&mut self) {
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

    async fn handle_start_event(&mut self) {
        if self.status == Status::Idle {
            self.current_step_start_time = Instant::now();
            self.status = Status::Running;
        } else if self.status == Status::Running {
            self.status = Status::Idle;
        }
        self.send_state().await;
    }

    async fn handle_right_turn(&mut self) {
        if self.status == Status::Idle {
            self.profile_index = (self.profile_index + 1) % 3;
        }

        self.target_temperature = PROFILES[self.profile_index].steps[0].end_temperature;
    }

    async fn handle_left_turn(&mut self) {
        if self.status == Status::Idle {
            if self.profile_index == 0 {
                self.profile_index = 2;
            } else {
                self.profile_index -= 1;
            }
        }
        self.target_temperature = PROFILES[self.profile_index].steps[0].end_temperature;
    }

    async fn handle_events(&mut self) {
        let receiver = EVENT_CHANNEL.receiver();
        if receiver.is_empty() {
            return;
        }
        match receiver.receive().await {
            Events::UpButtonPressed => {
                
            }
            Events::DownButtonPressed => {
                
            }
            Events::LeftButtonPressed => {
                
                self.handle_left_turn().await;
            }
            Events::RightButtonPressed => {
                
                self.handle_right_turn().await;
            }
            Events::CenterButtonPressed => {
               
                self.handle_start_event().await;
            }
        }
    }

    async fn handle_idle_state(&mut self) {
        self.handle_events().await;
        if CURRENT_TEMPERATURE.signaled() {
            let new_temperature = CURRENT_TEMPERATURE.wait().await;
            self.handle_new_temperature(new_temperature).await;
        }
        self.send_state().await;
    }

    async fn handle_init(&mut self, percent: u8) {

        self.target_temperature = PROFILES[self.profile_index].steps[0].end_temperature;

        if percent < 100 {
            Timer::after(Duration::from_millis(100)).await;
            self.status = Status::Initializing(percent + 1);
        } else {
            self.status = Status::Idle;
        }
        self.send_state().await;
    }

    async fn handle_timers(&mut self) {
        let step_time = PROFILES[self.profile_index].steps[self.current_step_index].time as u64;
        let now = Instant::now();
        let time_passed = now.duration_since(self.current_step_start_time).as_secs();

        // Prevent overflow
        let time_remaining = step_time.saturating_sub(time_passed) as u32;
        self.step_time_remaining = time_remaining;
        if time_remaining == 0 {
            self.current_step_start_time = Instant::now();
            self.current_step_index += 1;
            if self.current_step_index == PROFILES[self.profile_index].steps.len() {
                self.current_step_index = 0;
                self.handle_start_event().await;
            }
        }
    }

    async fn handle_running_state(&mut self) {
        self.handle_events().await;
        if CURRENT_TEMPERATURE.signaled() {
            let new_temperature = CURRENT_TEMPERATURE.wait().await;
            self.handle_new_temperature(new_temperature).await;
        }
        self.update_heater_power().await;
        self.handle_timers().await;
        self.send_state().await;
    }

    async fn update_heater_power(&mut self) {
        // Calculate the desired heater power based on the current and target temperatures
        if self.current_temperature < self.target_temperature {
            self.heater_power = 100;
        } else {
            self.heater_power = 0;
        }
    }

    async fn handle_new_temperature(&mut self, new_temperature: f32) {
        self.current_temperature = new_temperature;
    }
}
