use crate::pid::PidController;
use crate::profile::PROFILES;
use crate::temperature_sensor::CURRENT_TEMPERATURE;
use crate::{InputEvent, OutputCommand, ReflowControllerState, Status, CURRENT_STATE, INPUT_EVENT_CHANNEL, OUTPUT_COMMAND_CHANNEL};
use embassy_time::{Duration, Instant, Timer};
use {defmt_rtt as _, panic_probe as _};


pub struct ReflowController {
    target_temperature: f32,
    current_temperature: f32,
    door_closed: bool,
    fan: bool,
    light: bool,
    heater_power: u8, // value between 0 and 100
    profile_index: usize,
    current_step_index: usize,
    status: Status,
    current_step_start_time: Instant,
    profile_start_time: Instant,
    step_time_remaining: u32,
    pid_controller: PidController,
}


impl ReflowController {
    pub fn new() -> Self {
        Self {
            target_temperature: 0.0,
            current_temperature: -100.0,
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
            pid_controller: PidController::new(2.0, 0.5, 0.0, 0.1),
        }
    }

    pub async fn run(&mut self) {
        loop {
            if CURRENT_TEMPERATURE.signaled() {
                let new_temp = CURRENT_TEMPERATURE.wait().await;
                self.handle_new_temperature(new_temp).await;
            }
            // Check for input events
            let receiver = INPUT_EVENT_CHANNEL.receiver();

            if !receiver.is_empty() {
                let event = receiver.receive().await;
                self.handle_event(event).await;
            }
            match self.status {
                Status::Initializing => self.init().await,
                Status::Idle => self.idle().await,
                Status::Running => self.running().await,
                Status::Error => self.error().await,
            }
            let heater_sender = crate::HEATER_COMMAND_CHANNEL.sender();
            heater_sender.send(self.heater_power as u8).await;
            self.send_state();
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    async fn init(&mut self) {
        Timer::after_secs(1).await;
        self.status = Status::Idle;
    }

    async fn idle(&mut self) {
        if self.door_closed {
            OUTPUT_COMMAND_CHANNEL
                .sender()
                .send(OutputCommand::SetStartButtonLight(crate::LedState::LedOn))
                .await;
        } else {
            OUTPUT_COMMAND_CHANNEL
                .sender()
                .send(OutputCommand::SetStartButtonLight(crate::LedState::LedOff))
                .await;
        }
    }

    async fn enter_running_state(&mut self) {
        self.status = Status::Running;
        self.profile_start_time = Instant::now();
        self.current_step_start_time = Instant::now();
        self.current_step_index = 0;
        let current_step = &PROFILES[self.profile_index].steps[self.current_step_index];
        self.step_time_remaining = current_step.time;
        self.target_temperature = current_step.end_temperature;
    }

    async fn running(&mut self) {
        let current_step = &PROFILES[self.profile_index].steps[self.current_step_index];
        let elapsed = Instant::now() - self.current_step_start_time;
        if elapsed.as_secs() >= current_step.time as u64 {
            // Move to next step
            self.current_step_index += 1;
            if self.current_step_index >= PROFILES[self.profile_index].steps.len() {
                // Profile complete
                self.exit_running_state().await;
                return;
            }
            self.current_step_start_time = Instant::now();
            let next_step = &PROFILES[self.profile_index].steps[self.current_step_index];
            self.step_time_remaining = next_step.time;
            self.target_temperature = next_step.end_temperature;
        } else {
            self.step_time_remaining = current_step.time.saturating_sub(elapsed.as_secs() as u32);
        }

        self.heater_power =
            self.pid_controller
                .update(self.target_temperature, self.current_temperature) as u8;
    }

    async fn exit_running_state(&mut self) {
        self.status = Status::Idle;
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 0.0;
    }

    async fn error(&mut self) {
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 0.0;
        OUTPUT_COMMAND_CHANNEL
            .sender()
            .send(OutputCommand::SetStartButtonLight(crate::LedState::Blink(200, 200)))
            .await;
    }

    fn send_state(&mut self) {
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
        CURRENT_STATE.sender().send(state);
    }

    async fn handle_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::ButtonAPressed => {
                defmt::info!("Button One Pressed");
                // Handle button one press
            }
            InputEvent::ButtonBPressed => {
                defmt::info!("Button Two Pressed");
                // Handle button two press
            }
            InputEvent::StartButtonPressed => {
                if self.status == Status::Idle && self.door_closed {
                    self.enter_running_state().await;
                }
            }
            InputEvent::ButtonYPressed => {
                self.status = Status::Idle;
                self.heater_power = 0;
                self.fan = false;
                self.light = false;
                self.target_temperature = 0.0;
                // Handle button Y press
            }
            InputEvent::DoorOpened => {
                self.door_closed = false;
                if self.status == Status::Running {
                    self.status = Status::Error;
                    self.heater_power = 0;
                    self.fan = false;
                    self.light = false;
                    self.target_temperature = 0.0;
                }
            }
            InputEvent::DoorClosed => {
                self.door_closed = true;
            }
        }
    }

    async fn handle_new_temperature(&mut self, new_temperature: f32) {
        self.current_temperature = new_temperature;
    }
}

#[embassy_executor::task]
pub async fn controller_task() {
    let mut controller = ReflowController::new();
    controller.run().await;
}
