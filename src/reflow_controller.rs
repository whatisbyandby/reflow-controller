use crate::{log::*, SdCommand, SdResponse};
use embassy_time::{Instant, Timer};
use heapless::String;

use crate::{
    pid::PidController,
    profile::{Profile, StepName},
    HeaterCommand, SdProfileError,
};
use crate::{temperature_sensor::CURRENT_TEMPERATURE, HEATER_POWER};
use crate::{
    Event, OutputCommand, ReflowControllerState, Status, ACTIVE_PROFILE_CHANNEL, CURRENT_STATE,
    INPUT_EVENT_CHANNEL, OUTPUT_COMMAND_CHANNEL, PROFILE_LIST_CHANNEL, SD_COMMAND_CHANNEL,
    SD_RESPONSE_CHANNEL, SYSTEM_TICK_MILLIS,
};

pub struct ReflowController {
    target_temperature: f32,
    current_temperature: f32,
    step_start_temperature: f32, // temperature when current step began
    door_closed: bool,
    fan: bool,
    light: bool,
    heater_power: u8, // value between 0 and 100
    profile: Option<Profile>,
    current_step_index: usize,
    status: Status,
    profile_start_time: Instant,
    step_start_time: Instant,
    pid_controller: PidController,
    error_message: String<256>,
    sd_card_receiver: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        SdResponse,
        2,
    >,
}

impl ReflowController {
    pub fn new() -> Self {
        Self {
            target_temperature: -100.0,
            current_temperature: -100.0,
            step_start_temperature: -100.0,
            door_closed: false,
            fan: false,
            light: false,
            heater_power: 0,
            profile: None,
            current_step_index: 0,
            status: Status::Initializing,
            profile_start_time: Instant::now(),
            step_start_time: Instant::now(),
            // Sample time: (SYSTEM_TICK_MILLIS * 10) / 1000.0 = 1.0 second
            pid_controller: PidController::new(6.0, 0.05, 1.5, 1.0),
            error_message: String::new(),
            sd_card_receiver: SD_RESPONSE_CHANNEL.receiver(),
        }
    }

    pub async fn run(&mut self) {
        loop {
            // info!("Reflow Controller Tick");
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
                Status::PidTuning => self.pid_tuning().await,
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
        self.pid_controller.reset_integral();
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

    async fn enter_pid_tuning_state(&mut self) {
        self.status = Status::PidTuning;
        self.heater_power = 0;
        self.fan = false;
        self.light = true;
        self.target_temperature = 130.0;
        self.profile_start_time = Instant::now();
        // Reset PID integral term for clean tuning start
        self.pid_controller.reset_integral();
        OUTPUT_COMMAND_CHANNEL
            .sender()
            .send(OutputCommand::SetStartButtonLight(crate::LedState::Blink(
                SYSTEM_TICK_MILLIS * 3,
                SYSTEM_TICK_MILLIS * 3,
            )))
            .await;
    }

    async fn pid_tuning(&mut self) {
        // Hold constant temperature of 130C using PID
        self.heater_power = self
            .pid_controller
            .update(self.target_temperature, self.current_temperature);
    }

    async fn exit_pid_tuning_state(&mut self) {
        self.heater_power = 0;
        self.fan = true;
        self.light = false;
        self.target_temperature = 25.0;
        self.enter_idle_state();
    }

    async fn exit_finished_state(&mut self) {
        self.enter_idle_state();
    }

    async fn enter_running_state(&mut self) {
        self.status = Status::Running;
        self.fan = false;
        self.profile_start_time = Instant::now();
        self.current_step_index = 0;
        self.step_start_time = Instant::now();
        self.step_start_temperature = self.current_temperature;
        self.update_setpoint();
        // Reset PID integral term for clean profile start
        self.pid_controller.reset_integral();
    }

    fn step_completed(&self) -> bool {
        let profile = match &self.profile {
            Some(p) => p,
            None => return false,
        };
        let step = &profile.steps[self.current_step_index];
        let time_elapsed =
            (self.step_start_time.elapsed().as_millis() as u32 / (SYSTEM_TICK_MILLIS * 10)) as u32;
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
            let profile = self.profile.as_ref().unwrap(); // Safe: we can't be running without a profile
            if self.current_step_index + 1 < profile.steps.len() {
                self.fan = profile.steps[self.current_step_index].has_fan;
                self.current_step_index += 1;
                self.step_start_time = Instant::now();
                self.step_start_temperature = self.current_temperature;
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
        self.target_temperature = 25.0;
    }

    fn exit_error_state(&mut self) {
        self.status = Status::Idle;
        self.heater_power = 0;
        self.fan = false;
        self.light = false;
        self.target_temperature = 25.0;
        self.error_message.clear();
    }

    fn send_state(&mut self) {
        let (kp, ki, kd) = self.pid_controller.get_parameters();
        let (p_term, i_term, d_term) = self.pid_controller.get_last_terms();

        // Get profile name and step name, or use defaults if no profile loaded
        let (profile_name, step_name) = match &self.profile {
            Some(profile) => (
                profile.name.clone(),
                profile.steps[self.current_step_index].step_name.to_str(),
            ),
            None => {
                let mut empty = heapless::String::<32>::new();
                let _ = empty.push_str("No Profile");
                (empty, "N/A")
            }
        };

        let state = ReflowControllerState {
            status: self.status.clone(),
            target_temperature: self.target_temperature,
            current_temperature: self.current_temperature,
            door_closed: self.door_closed,
            fan: self.fan,
            light: self.light,
            heater_power: self.heater_power,
            timer: if self.status == Status::Idle || self.status == Status::Error {
                0
            } else if self.status == Status::PidTuning {
                self.get_elapsed_time_ms(self.profile_start_time)
            } else {
                self.get_elapsed_time_ms(self.profile_start_time)
            },
            current_profile: profile_name,
            current_step: step_name,
            error_message: self.error_message.clone(),
            pid_kp: kp,
            pid_ki: ki,
            pid_kd: kd,
            pid_p_term: p_term,
            pid_i_term: i_term,
            pid_d_term: d_term,
        };
        CURRENT_STATE.sender().send(state);
    }

    fn get_elapsed_time_ms(&self, instant: Instant) -> u32 {
        (self.profile_start_time.elapsed().as_millis() as u32) * (SYSTEM_TICK_MILLIS / 100)
    }

    fn update_setpoint(&mut self) {
        let profile = match &self.profile {
            Some(p) => p,
            None => return, // No profile loaded, can't update setpoint
        };
        let step = &profile.steps[self.current_step_index];

        // Calculate elapsed time in seconds since step started
        let elapsed_ms = self.step_start_time.elapsed().as_millis() as u32;
        let elapsed_seconds = elapsed_ms as f32 / 1000.0;

        // Calculate the total temperature change needed for this step
        let temp_delta = step.set_temperature - self.step_start_temperature;

        // Calculate target temperature based on maximum allowed rate of change
        // max_rate is in degrees Celsius per second
        let max_temp_change = step.max_rate * elapsed_seconds;

        // Calculate the target temperature with rate limiting
        let rate_limited_target = if temp_delta >= 0.0 {
            // Heating: ramp up at max_rate, but don't exceed set_temperature
            (self.step_start_temperature + max_temp_change).min(step.set_temperature)
        } else {
            // Cooling: ramp down at max_rate, but don't go below set_temperature
            (self.step_start_temperature - max_temp_change).max(step.set_temperature)
        };

        self.target_temperature = rate_limited_target;
    }

    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::StartCommand => {
                if self.profile.is_none() {
                    info!("Cannot start: no profile loaded");
                    self.enter_error_state("No profile loaded. Load a profile first.")
                        .await;
                } else if self.status == Status::Idle && self.door_closed {
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
                } else if self.status == Status::PidTuning {
                    info!("Stopping PID tuning mode");
                    self.exit_pid_tuning_state().await;
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
                    // Check if we can safely open door during cooling
                    let is_cooling = self
                        .profile
                        .as_ref()
                        .map(|p| p.steps[self.current_step_index].step_name == StepName::Cooling)
                        .unwrap_or(false);

                    if !is_cooling {
                        info!("Door opened while running, entering error state");
                        self.enter_error_state("Door opened while running!").await;
                    } else {
                        info!("Door opened during cooling step, stopping reflow process");
                    }
                }
            }
            Event::LoadProfile(filename) => {
                info!("Load Profile Event");
                if self.status == Status::Idle || self.status == Status::Error {
                    SD_COMMAND_CHANNEL
                        .sender()
                        .send(SdCommand::ReadProfile { filename })
                        .await;

                    let response = self.sd_card_receiver.receive().await;

                    info!("Received SD card response");

                    match response {
                        SdResponse::ProfileData(profile) => {
                            info!("Profile loaded: {}", profile.name.as_str());
                            let sender = ACTIVE_PROFILE_CHANNEL.sender();
                            sender.send(profile.clone()).await;
                            self.profile = Some(profile);
                        }
                        SdResponse::Error(_err) => {
                            info!("Failed to load profile");
                            self.enter_error_state("Failed to load profile from SD card")
                                .await;
                        }
                        _ => {
                            info!("Unexpected response when loading profile");
                            self.enter_error_state("Unexpected error loading profile")
                                .await;
                        }
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
                        info!("Error listing profiles");
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
            Event::FanControl(on) => {
                info!("Manual fan control: {}", if on { "ON" } else { "OFF" });
                self.fan = on;
            }
            Event::WriteProfile {
                filename,
                profile_json,
            } => {
                info!("Writing profile to file: {}", filename.as_str());
            }
            Event::StartPidTuning => {
                if self.status == Status::Idle && self.door_closed {
                    info!("Starting PID tuning mode");
                    self.enter_pid_tuning_state().await;
                } else if self.status == Status::PidTuning {
                    info!("Stopping PID tuning mode");
                    self.exit_pid_tuning_state().await;
                } else {
                    info!("Cannot start PID tuning: either not idle or door is open");
                }
            }
        }
    }

    async fn handle_new_temperature(&mut self, new_temperature: f32) {
        self.current_temperature = new_temperature;
    }

    pub async fn get_available_profiles(
        &self,
    ) -> Result<heapless::Vec<heapless::String<64>, 16>, SdProfileError> {
        SD_COMMAND_CHANNEL
            .sender()
            .send(SdCommand::ListProfiles)
            .await;

        let response = self.sd_card_receiver.receive().await;
        match response {
            SdResponse::ProfileList(profiles) => Ok(profiles),
            SdResponse::Error(err) => Err(err),
            _ => Err(SdProfileError::Unknown),
        }
    }

    // Getter methods for testing
    #[cfg(test)]
    pub fn status(&self) -> &Status {
        &self.status
    }

    #[cfg(test)]
    pub fn heater_power(&self) -> u8 {
        self.heater_power
    }

    #[cfg(test)]
    pub fn fan(&self) -> bool {
        self.fan
    }

    #[cfg(test)]
    pub fn light(&self) -> bool {
        self.light
    }

    #[cfg(test)]
    pub fn door_closed(&self) -> bool {
        self.door_closed
    }

    #[cfg(test)]
    pub fn set_door_closed(&mut self, closed: bool) {
        self.door_closed = closed;
    }

    #[cfg(test)]
    pub fn current_step_index(&self) -> usize {
        self.current_step_index
    }

    #[cfg(test)]
    pub fn target_temperature(&self) -> f32 {
        self.target_temperature
    }

    #[cfg(test)]
    pub fn current_temperature(&self) -> f32 {
        self.current_temperature
    }

    #[cfg(test)]
    pub fn set_current_temperature(&mut self, temp: f32) {
        self.current_temperature = temp;
    }

    #[cfg(test)]
    pub fn profile(&self) -> &Option<Profile> {
        &self.profile
    }

    #[cfg(test)]
    pub fn error_message(&self) -> &heapless::String<256> {
        &self.error_message
    }

    #[cfg(test)]
    pub fn enter_idle_state_test(&mut self) {
        self.enter_idle_state();
    }

    #[cfg(test)]
    pub fn exit_error_state_test(&mut self) {
        self.exit_error_state();
    }

    #[cfg(test)]
    pub fn step_completed_test(&self) -> bool {
        self.step_completed()
    }
}

#[embassy_executor::task]
pub async fn controller_task() {
    let mut controller = ReflowController::new();
    controller.run().await;
}
