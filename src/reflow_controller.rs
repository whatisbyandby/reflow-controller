use crate::log::*;
use embassy_time::{Instant, Timer};
use heapless::String;

use crate::{
    pid::PidController,
    profile::{create_default_profile, Profile, StepName},
    sd_profile_reader::{SdProfileError, SdProfileReader},
    HeaterCommand,
};
use crate::{temperature_sensor::CURRENT_TEMPERATURE, HEATER_POWER};
use crate::{
    Event, OutputCommand, ReflowControllerState, Status, ACTIVE_PROFILE_CHANNEL, CURRENT_STATE,
    INPUT_EVENT_CHANNEL, OUTPUT_COMMAND_CHANNEL, PROFILE_LIST_CHANNEL, SYSTEM_TICK_MILLIS,
};

pub struct ReflowController {
    target_temperature: f32,
    current_temperature: f32,
    door_closed: bool,
    fan: bool,
    light: bool,
    heater_power: u8, // value between 0 and 100
    profile: Profile,
    current_step_index: usize,
    status: Status,
    profile_start_time: Instant,
    step_start_time: Instant,
    pid_controller: PidController,
    error_message: String<256>,
    sd_reader: SdProfileReader,
}

impl ReflowController {
    pub fn new() -> Self {
        Self {
            target_temperature: -100.0,
            current_temperature: -100.0,
            door_closed: false,
            fan: false,
            light: false,
            heater_power: 0,
            profile: create_default_profile(),
            current_step_index: 0,
            status: Status::Initializing,
            profile_start_time: Instant::now(),
            step_start_time: Instant::now(),
            pid_controller: PidController::new(3.0, 0.5, 0.0),
            error_message: String::new(),
            sd_reader: SdProfileReader::new(),
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
                Status::Finished => self.finished().await,
            }
            let heater_sender = HEATER_POWER.sender();
            heater_sender.send(HeaterCommand::SetFan(self.fan)).await;
            heater_sender
                .send(crate::HeaterCommand::SetPower(self.heater_power))
                .await;
            self.send_state();
            Timer::after_millis((SYSTEM_TICK_MILLIS * 10).into()).await;
        }
    }

    async fn init(&mut self) {
        Timer::after_millis((SYSTEM_TICK_MILLIS * 10).into()).await; // 1 second in simulation time
        self.enter_idle_state();
    }

    fn enter_idle_state(&mut self) {
        self.status = Status::Idle;
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 25.0;
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

    async fn enter_finished_state(&mut self) {
        self.status = Status::Finished;
        self.heater_power = 0;
        self.fan = true;
        self.light = false;
        self.target_temperature = 25.0;
        OUTPUT_COMMAND_CHANNEL
            .sender()
            .send(OutputCommand::SetStartButtonLight(crate::LedState::Blink(
                SYSTEM_TICK_MILLIS * 5,
                SYSTEM_TICK_MILLIS * 5,
            )))
            .await;
    }

    async fn finished(&mut self) {
        // Wait for user to reset
        Timer::after_millis((SYSTEM_TICK_MILLIS * 10).into()).await; // 1 second in simulation time
    }

    async fn exit_finished_state(&mut self) {
        self.enter_idle_state();
    }

    async fn enter_running_state(&mut self) {
        self.status = Status::Running;
        self.fan = false;
        self.profile_start_time = Instant::now();
        self.current_step_index = 0;
        self.update_setpoint();
        // Reset PID integral term for clean profile start
        self.pid_controller.reset_integral();
    }

    fn step_completed(&self) -> bool {
        let step = &self.profile.steps[self.current_step_index];
        let time_elapsed =
            (self.step_start_time.elapsed().as_millis() as u32 / SYSTEM_TICK_MILLIS) as u32;
        let step_end_time = step.step_time;
        let temp_reached = if step.is_cooling {
            self.current_temperature <= step.set_temperature
        } else {
            self.current_temperature >= (step.set_temperature - 1.0) // Allow small overshoot margin
        };
        time_elapsed >= step_end_time && temp_reached
    }

    async fn running(&mut self) {
        // Check if we've reached the target temperature for the current step
        self.update_setpoint();
        if self.step_completed() {
            // Move to the next step if available
            if self.current_step_index + 1 < self.profile.steps.len() {
                self.fan = self.profile.steps[self.current_step_index].has_fan;
                self.current_step_index += 1;
                self.step_start_time = Instant::now();
                self.update_setpoint();
                // Reset PID integral term for clean step transition
                self.pid_controller.reset_integral();
            } else {
                // Completed all steps
                self.exit_running_state().await;
                self.enter_finished_state().await;
                return;
            }
        }
        self.heater_power = self
            .pid_controller
            .update(self.target_temperature, self.current_temperature);
    }

    async fn exit_running_state(&mut self) {
        self.heater_power = 0;
        self.fan = true;
        self.light = false;
        self.target_temperature = 25.0;
    }

    async fn enter_error_state(&mut self, message: &str) {
        self.error_message.clear();
        let _ = self.error_message.push_str(message);
        self.status = Status::Error;
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 0.0;
        OUTPUT_COMMAND_CHANNEL
            .sender()
            .send(OutputCommand::SetStartButtonLight(crate::LedState::Blink(
                SYSTEM_TICK_MILLIS * 2,
                SYSTEM_TICK_MILLIS * 2,
            )))
            .await;
    }

    async fn error(&mut self) {
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 0.0;
    }

    fn exit_error_state(&mut self) {
        self.status = Status::Idle;
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 0.0;
        self.error_message.clear();
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
            timer: if self.status == Status::Idle {
                0
            } else {
                self.profile_start_time.elapsed().as_millis() as u32 / SYSTEM_TICK_MILLIS
            },
            current_profile: self.profile.name.clone(),
            current_step: self.profile.steps[self.current_step_index]
                .step_name
                .to_str(),
            error_message: self.error_message.clone(),
        };
        CURRENT_STATE.sender().send(state);
    }

    fn update_setpoint(&mut self) {
        #[cfg(feature = "ramp_setpoint")]
        {
            if self.target_temperature < 26.0 {
                self.target_temperature = self.current_temperature;
            }

            let step_temperature = self.profile.steps[self.current_step_index].set_temperature;
            let difference = step_temperature - self.current_temperature;
            let set_temp_diff = self.profile.steps[self.current_step_index].set_temperature
                - self.target_temperature;
            let time_remaining = self.profile.steps[self.current_step_index]
                .target_time
                .saturating_sub(self.profile_start_time.elapsed().as_secs() as u32);
            if time_remaining > 0 && set_temp_diff > 0.0 {
                let adjustment = difference / time_remaining as f32;
                self.target_temperature = self.target_temperature + adjustment;
            } else {
                self.target_temperature = step_temperature;
            }
        }

        #[cfg(not(feature = "ramp_setpoint"))]
        {
            self.target_temperature = self.profile.steps[self.current_step_index].set_temperature;
        }
    }

    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::StartCommand => {
                if self.status == Status::Idle && self.door_closed {
                    info!("Starting reflow process");
                    self.enter_running_state().await;
                } else {
                    info!("Cannot start: either not idle or door is open");
                }
            }
            Event::StopCommand => {
                if self.status == Status::Running {
                    info!("Stopping reflow process");
                    self.exit_running_state().await;
                    self.enter_idle_state();
                }
            }
            Event::ResetCommand => {
                if self.status == Status::Finished {
                    info!("Resetting to idle state");
                    self.exit_finished_state().await;
                }
                if self.status == Status::Error {
                    info!("Resetting from error state to idle");
                    self.exit_error_state();
                }
            }
            Event::DoorStateChanged(closed) => {
                self.door_closed = closed;
                if !closed && self.status == Status::Running {
                    if self.profile.steps[self.current_step_index].step_name != StepName::Cooling {
                        info!("Door opened while running, entering error state");
                        self.enter_error_state("Door opened while running!").await;
                    } else {
                        info!("Door opened during cooling step, stopping reflow process");
                    }
                }
            }
            Event::LoadProfile(filename) => {
                if self.status == Status::Idle {
                    info!("Loading profile: {}", filename.as_str());
                    match self.sd_reader.read_profile(filename.as_str()).await {
                        Ok(profile) => {
                            info!("Successfully loaded profile: {}", profile.name.as_str());
                            self.profile = profile.clone();
                            // Send active profile over USB
                            let sender = ACTIVE_PROFILE_CHANNEL.sender();
                            sender.send(profile).await;
                        }
                        Err(err) => match err {
                            SdProfileError::FileNotFound => {
                                self.enter_error_state("Profile file not found").await;
                            }
                            SdProfileError::ParseError => {
                                self.enter_error_state("Profile parse error").await;
                            }
                            SdProfileError::InvalidFormat => {
                                self.enter_error_state("Invalid profile format").await;
                            }
                            SdProfileError::SdCardError => {
                                self.enter_error_state("SD card error").await;
                            }
                            SdProfileError::TooManyProfiles => {
                                self.enter_error_state("Too many profiles").await;
                            }
                        },
                    }
                } else {
                    info!("Cannot load profile: not in idle state");
                }
            }
            Event::ListProfilesRequest => {
                info!("Listing available profiles");
                match self.get_available_profiles().await {
                    Ok(profiles) => {
                        let sender = PROFILE_LIST_CHANNEL.sender();
                        sender.send(profiles).await;
                    }
                    Err(err) => {
                        info!("Error listing profiles: {:?}", err);
                        // Send empty list on error
                        let sender = PROFILE_LIST_CHANNEL.sender();
                        let empty_list = heapless::Vec::new();
                        sender.send(empty_list).await;
                    }
                }
            }
            Event::SimulationReset => {
                info!("Triggering simulation reset");
                let heater_sender = HEATER_POWER.sender();
                heater_sender.send(HeaterCommand::SimulationReset).await;
            }
            Event::UpdatePidParameters { kp, ki, kd } => {
                info!("Updating PID parameters: Kp={}, Ki={}, Kd={}", kp, ki, kd);
                // Update PID controller parameters with integral reset for stability
                self.pid_controller.update_parameters(kp, ki, kd, true);

                // Also send to heater task for logging (though it doesn't use PID directly)
                let heater_sender = HEATER_POWER.sender();
                heater_sender
                    .send(HeaterCommand::UpdatePidParameters { kp, ki, kd })
                    .await;
            }
        }
        self.send_state();
    }

    async fn handle_new_temperature(&mut self, new_temperature: f32) {
        self.current_temperature = new_temperature;
    }

    pub async fn get_available_profiles(
        &self,
    ) -> Result<heapless::Vec<heapless::String<64>, 16>, SdProfileError> {
        self.sd_reader.list_profiles().await
    }

    pub async fn init_sd_card(&mut self) -> Result<(), SdProfileError> {
        self.sd_reader.init().await
    }
}

#[embassy_executor::task]
pub async fn controller_task() {
    let mut controller = ReflowController::new();
    controller.run().await;
}
