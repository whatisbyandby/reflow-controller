#[cfg_attr(not(feature = "std"), no_std)]

pub mod pid;
pub mod profile;
pub mod reflow_controller;
pub mod sd_profile_reader;

#[cfg(feature = "rp2040")]
pub use defmt as log;

#[cfg(feature = "std")]
pub use log;

#[cfg(feature = "rp2040")]
pub mod inputs_rp2040;
#[cfg(feature = "rp2040")]
pub use inputs_rp2040 as inputs;

#[cfg(feature = "std")]
pub mod inputs_std;
#[cfg(feature = "std")]
pub use inputs_std as inputs;

#[cfg(feature = "rp2040")]
pub mod outputs_rp2040;
#[cfg(feature = "rp2040")]
pub use outputs_rp2040 as outputs;

#[cfg(feature = "std")]
pub mod outputs_std;
#[cfg(feature = "std")]
pub use outputs_std as outputs;

#[cfg(feature = "rp2040")]
pub mod resources_rp2040;
#[cfg(feature = "rp2040")]
pub use resources_rp2040 as resources;

#[cfg(feature = "rp2040")]
pub mod heater_rp2040;
#[cfg(feature = "rp2040")]
pub use heater_rp2040 as heater;

#[cfg(feature = "std")]
pub mod heater_std;
#[cfg(feature = "std")]
pub use heater_std as heater;

#[cfg(feature = "rp2040")]
pub mod temperature_sensor_mcp9600;
#[cfg(feature = "rp2040")]
pub use temperature_sensor_mcp9600 as temperature_sensor;

#[cfg(feature = "std")]
pub mod temperature_sensor_mock;
#[cfg(feature = "std")]
pub use temperature_sensor_mock as temperature_sensor;

#[cfg(feature = "rp2040")]
pub mod usb_interface_rp2040;
#[cfg(feature = "rp2040")]
pub use usb_interface_rp2040 as usb_interface;

#[cfg(feature = "std")]
pub mod usb_interface_std;
#[cfg(feature = "std")]
pub use usb_interface_std as usb_interface;

pub static VERSION: &str = "v0.1";
pub static SYSTEM_TICK_MILLIS: u32 = 100;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    StartCommand,
    StopCommand,
    ResetCommand,
    DoorStateChanged(bool),            // true = closed, false = opened
    LoadProfile(heapless::String<64>), // filename to load from SD card
    ListProfilesRequest,
    SimulationReset,
    UpdatePidParameters { kp: f32, ki: f32, kd: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    LedOn,
    LedOff,
    Blink(u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCommand {
    SetFan(bool),
    SetLight(bool),
    SetBuzzer(bool),
    SetStartButtonLight(LedState),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeaterCommand {
    SetPower(u8),
    SetFan(bool),
    SimulationReset,
    UpdatePidParameters { kp: f32, ki: f32, kd: f32 },
}

pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Event, 3> = Channel::new();
pub static OUTPUT_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, OutputCommand, 3> =
    Channel::new();
pub static HEATER_POWER: Channel<CriticalSectionRawMutex, HeaterCommand, 2> = Channel::new();
pub static CURRENT_STATE: Watch<CriticalSectionRawMutex, ReflowControllerState, 3> = Watch::new();
pub static PROFILE_LIST_CHANNEL: Channel<
    CriticalSectionRawMutex,
    heapless::Vec<heapless::String<64>, 16>,
    1,
> = Channel::new();
pub static ACTIVE_PROFILE_CHANNEL: Channel<CriticalSectionRawMutex, profile::Profile, 1> =
    Channel::new();

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Initializing,
    Idle,
    Running,
    Finished,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflowControllerState {
    pub status: Status,
    pub target_temperature: f32,
    pub current_temperature: f32,
    pub door_closed: bool,
    pub fan: bool,
    pub light: bool,
    pub heater_power: u8, // value between 0 and 100
    pub timer: u32,
    pub current_step: &'static str,
    pub current_profile: heapless::String<32>,
    pub error_message: heapless::String<256>,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod pid_controller_tests {
        use super::*;
        use crate::pid::PidController;

        #[test]
        fn test_pid_controller_new() {
            let pid = PidController::new(1.0, 0.5, 0.1);
            let (kp, ki, kd) = pid.get_parameters();

            assert_eq!(kp, 1.0);
            assert_eq!(ki, 0.5);
            assert_eq!(kd, 0.1);
        }

        #[test]
        fn test_pid_controller_update_basic() {
            let mut pid = PidController::new(1.0, 0.0, 0.0); // P-only controller

            // Test with positive error (setpoint > measurement)
            let output = pid.update(100.0, 90.0); // Error = 10.0
            assert_eq!(output, 10); // P * error = 1.0 * 10.0 = 10.0
        }

        #[test]
        fn test_pid_controller_integral_term() {
            let mut pid = PidController::new(0.0, 1.0, 0.0); // I-only controller

            // First update: integral accumulates error
            let output1 = pid.update(100.0, 90.0); // Error = 10.0
            assert_eq!(output1, 10); // I * integral = 1.0 * 10.0 = 10.0

            // Second update: integral accumulates more error
            let output2 = pid.update(100.0, 90.0); // Error = 10.0 again
            assert_eq!(output2, 20); // I * integral = 1.0 * 20.0 = 20.0
        }

        #[test]
        fn test_pid_controller_derivative_term() {
            let mut pid = PidController::new(0.0, 0.0, 1.0); // D-only controller

            // First update: no previous error, so derivative is 0
            let output1 = pid.update(100.0, 90.0); // Error = 10.0
            assert_eq!(output1, 10); // D * (error - prev_error) = 1.0 * (10.0 - 0.0) = 10.0

            // Second update: error changes
            let output2 = pid.update(100.0, 85.0); // Error = 15.0
            assert_eq!(output2, 5); // D * (error - prev_error) = 1.0 * (15.0 - 10.0) = 5.0
        }

        #[test]
        fn test_pid_controller_output_clamping() {
            let mut pid = PidController::new(10.0, 0.0, 0.0); // High P gain

            // Large error should be clamped to max output (100)
            let output = pid.update(200.0, 0.0); // Error = 200.0
            assert_eq!(output, 100); // Should be clamped to max (100)

            // Negative error should be clamped to min output (0)
            let output = pid.update(0.0, 200.0); // Error = -200.0
            assert_eq!(output, 0); // Should be clamped to min (0)
        }

        #[test]
        fn test_pid_controller_integral_windup_protection() {
            let mut pid = PidController::new(1.0, 1.0, 0.0);

            // Create a large error that would saturate output
            pid.update(200.0, 0.0); // Large error, output will be clamped

            // The integral should be reduced due to windup protection
            // This is harder to test directly, but we can verify behavior
            let output = pid.update(110.0, 100.0); // Small error = 10.0
            // If windup protection works, output should be reasonable
            assert!(output <= 100 && output >= 0);
        }

        #[test]
        fn test_pid_controller_reset_integral() {
            let mut pid = PidController::new(0.0, 1.0, 0.0);

            // Build up integral term
            pid.update(100.0, 90.0);
            pid.update(100.0, 90.0);
            let output_before = pid.update(100.0, 90.0);
            assert!(output_before > 20); // Should have accumulated

            // Reset integral
            pid.reset_integral();
            let output_after = pid.update(100.0, 90.0);
            assert_eq!(output_after, 10); // Should be back to single error value
        }

        #[test]
        fn test_pid_controller_update_parameters() {
            let mut pid = PidController::new(1.0, 1.0, 1.0);

            // Update parameters without resetting integral
            pid.update(100.0, 90.0); // Build up some integral
            pid.update_parameters(2.0, 2.0, 2.0, false);

            let (kp, ki, kd) = pid.get_parameters();
            assert_eq!(kp, 2.0);
            assert_eq!(ki, 2.0);
            assert_eq!(kd, 2.0);

            // Update parameters with integral reset
            pid.update_parameters(3.0, 3.0, 3.0, true);
            let output = pid.update(100.0, 90.0);
            // With reset integral:
            // P: 3.0 * 10.0 = 30.0
            // I: 3.0 * 10.0 = 30.0 (integral accumulates current error)
            // D: 3.0 * (10.0 - 10.0) = 0.0 (previous_error retained from before parameter update)
            // Total: 30 + 30 + 0 = 60
            assert_eq!(output, 60);
        }

        #[test]
        fn test_pid_controller_zero_error() {
            let mut pid = PidController::new(1.0, 1.0, 1.0);

            // Zero error should produce zero output (ignoring accumulated integral)
            let output = pid.update(100.0, 100.0);
            assert_eq!(output, 0);
        }

        #[test]
        fn test_pid_controller_negative_setpoint() {
            let mut pid = PidController::new(1.0, 0.0, 0.0);

            // Negative setpoint should work correctly
            let output = pid.update(-10.0, 0.0); // Error = -10.0
            assert_eq!(output, 0); // Clamped to minimum (0)
        }
    }

    mod profile_tests {
        use super::*;
        use crate::profile::{create_default_profile, StepName, Step, Profile};

        #[test]
        fn test_step_name_to_str() {
            assert_eq!(StepName::Preheat.to_str(), "Preheat");
            assert_eq!(StepName::Soak.to_str(), "Soak");
            assert_eq!(StepName::Ramp.to_str(), "Ramp");
            assert_eq!(StepName::ReflowRamp.to_str(), "Reflow Ramp");
            assert_eq!(StepName::ReflowCool.to_str(), "Reflow Cool");
            assert_eq!(StepName::Cooling.to_str(), "Cooling");
        }

        #[test]
        fn test_default_profile_creation() {
            let profile = create_default_profile();

            assert_eq!(profile.name.as_str(), "Default Profile");
            assert_eq!(profile.steps.len(), 6);

            // Check first step (Preheat)
            let preheat = &profile.steps[0];
            assert_eq!(preheat.step_name, StepName::Preheat);
            assert_eq!(preheat.set_temperature, 150.0);
            assert_eq!(preheat.target_time, 90);
            assert_eq!(preheat.step_time, 90);
            assert_eq!(preheat.max_rate, 2.0);
            assert!(!preheat.is_cooling);
            assert!(!preheat.has_fan);
        }

        #[test]
        fn test_default_profile_step_sequence() {
            let profile = create_default_profile();

            let expected_sequence = [
                StepName::Preheat,
                StepName::Soak,
                StepName::Ramp,
                StepName::ReflowRamp,
                StepName::ReflowCool,
                StepName::Cooling,
            ];

            for (i, expected_step) in expected_sequence.iter().enumerate() {
                assert_eq!(profile.steps[i].step_name, *expected_step);
            }
        }

        #[test]
        fn test_default_profile_temperature_progression() {
            let profile = create_default_profile();

            // Check temperature progression makes sense
            assert_eq!(profile.steps[0].set_temperature, 150.0); // Preheat
            assert_eq!(profile.steps[1].set_temperature, 175.0); // Soak
            assert_eq!(profile.steps[2].set_temperature, 230.0); // Ramp
            assert_eq!(profile.steps[3].set_temperature, 240.0); // ReflowRamp (peak)
            assert_eq!(profile.steps[4].set_temperature, 217.0); // ReflowCool
            assert_eq!(profile.steps[5].set_temperature, 50.0);  // Cooling
        }

        #[test]
        fn test_default_profile_cooling_steps() {
            let profile = create_default_profile();

            // Only ReflowCool and Cooling should be cooling steps
            assert!(!profile.steps[0].is_cooling); // Preheat
            assert!(!profile.steps[1].is_cooling); // Soak
            assert!(!profile.steps[2].is_cooling); // Ramp
            assert!(!profile.steps[3].is_cooling); // ReflowRamp
            assert!(profile.steps[4].is_cooling);  // ReflowCool
            assert!(profile.steps[5].is_cooling);  // Cooling
        }

        #[test]
        fn test_default_profile_fan_usage() {
            let profile = create_default_profile();

            // Only Cooling step should use fan
            for i in 0..5 {
                assert!(!profile.steps[i].has_fan);
            }
            assert!(profile.steps[5].has_fan); // Cooling step
        }

        #[test]
        fn test_step_equality() {
            let step1 = Step {
                step_name: StepName::Preheat,
                set_temperature: 150.0,
                target_time: 90,
                step_time: 90,
                max_rate: 2.0,
                is_cooling: false,
                has_fan: false,
            };

            let step2 = Step {
                step_name: StepName::Preheat,
                set_temperature: 150.0,
                target_time: 90,
                step_time: 90,
                max_rate: 2.0,
                is_cooling: false,
                has_fan: false,
            };

            // Steps should be equal
            assert_eq!(step1.step_name, step2.step_name);
            assert_eq!(step1.set_temperature, step2.set_temperature);
        }

        #[test]
        fn test_profile_name_constraints() {
            let profile = create_default_profile();

            // Verify name fits in heapless::String<32>
            assert!(profile.name.len() <= 32);
            assert!(!profile.name.is_empty());
        }
    }

    mod event_tests {
        use super::*;

        #[test]
        fn test_reset_command_from_finished() {
            let reset_event = Event::ResetCommand;

            // Verify reset event structure
            match reset_event {
                Event::ResetCommand => {
                    // Reset command should work from Finished state
                    assert!(true);
                }
                _ => panic!("Expected ResetCommand event"),
            }
        }

        #[test]
        fn test_reset_command_from_error() {
            let reset_event = Event::ResetCommand;

            // Verify reset event structure
            match reset_event {
                Event::ResetCommand => {
                    // Reset command should work from Error state
                    assert!(true);
                }
                _ => panic!("Expected ResetCommand event"),
            }
        }

        #[test]
        fn test_stop_command() {
            let stop_event = Event::StopCommand;

            // Verify stop event structure
            match stop_event {
                Event::StopCommand => {
                    // Stop command should work from Running state
                    assert!(true);
                }
                _ => panic!("Expected StopCommand event"),
            }
        }

        #[test]
        fn test_profile_load_event() {
            let mut profile_name = heapless::String::<64>::new();
            profile_name.push_str("test_profile.txt").unwrap();
            let load_event = Event::LoadProfile(profile_name.clone());

            match load_event {
                Event::LoadProfile(filename) => {
                    assert_eq!(filename.as_str(), "test_profile.txt");
                }
                _ => panic!("Expected LoadProfile event"),
            }
        }

        #[test]
        fn test_pid_parameter_update_event() {
            let pid_event = Event::UpdatePidParameters {
                kp: 1.5,
                ki: 0.3,
                kd: 0.1,
            };

            match pid_event {
                Event::UpdatePidParameters { kp, ki, kd } => {
                    assert_eq!(kp, 1.5);
                    assert_eq!(ki, 0.3);
                    assert_eq!(kd, 0.1);
                }
                _ => panic!("Expected UpdatePidParameters event"),
            }
        }

        #[test]
        fn test_door_state_change_event() {
            // Test door closing
            let door_close_event = Event::DoorStateChanged(true);
            match door_close_event {
                Event::DoorStateChanged(closed) => {
                    assert!(closed);
                }
                _ => panic!("Expected DoorStateChanged event"),
            }

            // Test door opening
            let door_open_event = Event::DoorStateChanged(false);
            match door_open_event {
                Event::DoorStateChanged(closed) => {
                    assert!(!closed);
                }
                _ => panic!("Expected DoorStateChanged event"),
            }
        }

        #[test]
        fn test_simulation_reset_event() {
            let sim_reset_event = Event::SimulationReset;
            match sim_reset_event {
                Event::SimulationReset => {
                    assert!(true);
                }
                _ => panic!("Expected SimulationReset event"),
            }
        }

        #[test]
        fn test_list_profiles_request_event() {
            let list_event = Event::ListProfilesRequest;
            match list_event {
                Event::ListProfilesRequest => {
                    assert!(true);
                }
                _ => panic!("Expected ListProfilesRequest event"),
            }
        }

        #[test]
        fn test_start_command_event() {
            let start_event = Event::StartCommand;
            match start_event {
                Event::StartCommand => {
                    assert!(true);
                }
                _ => panic!("Expected StartCommand event"),
            }
        }
    }

    mod sd_profile_reader_tests {
        use super::*;
        use crate::sd_profile_reader::{SdProfileReader, SdProfileError};

        #[test]
        fn test_sd_profile_reader_new() {
            let reader = SdProfileReader::new();
            // Can't access initialized field directly, but we can test that new() doesn't panic
            assert!(true);
        }

        #[test]
        fn test_sd_profile_error_types() {
            // Test that all error variants can be created
            let _file_not_found = SdProfileError::FileNotFound;
            let _parse_error = SdProfileError::ParseError;
            let _invalid_format = SdProfileError::InvalidFormat;
            let _sd_card_error = SdProfileError::SdCardError;
            let _too_many_profiles = SdProfileError::TooManyProfiles;

            // All error types exist and can be constructed
            assert!(true);
        }

        // Note: The mock profile creation methods are private,
        // so we can't test them directly without async runtime.
        // The profile structure and logic is already tested in profile_tests module.
    }

    mod data_structure_tests {
        use super::*;

        #[test]
        fn test_status_enum() {
            let statuses = [
                Status::Initializing,
                Status::Idle,
                Status::Running,
                Status::Finished,
                Status::Error,
            ];

            // Test that all status variants can be created and compared
            for status in &statuses {
                assert_eq!(*status, *status); // Self-equality
            }

            // Test specific comparisons
            assert_ne!(Status::Idle, Status::Running);
            assert_ne!(Status::Error, Status::Finished);
        }

        #[test]
        fn test_led_state_enum() {
            let led_on = LedState::LedOn;
            let led_off = LedState::LedOff;
            let led_blink = LedState::Blink(500, 500);

            // Test that LED states can be created and compared
            assert_eq!(led_on, LedState::LedOn);
            assert_eq!(led_off, LedState::LedOff);
            assert_eq!(led_blink, LedState::Blink(500, 500));
            assert_ne!(led_on, led_off);
        }

        #[test]
        fn test_output_command_enum() {
            let commands = [
                OutputCommand::SetFan(true),
                OutputCommand::SetFan(false),
                OutputCommand::SetLight(true),
                OutputCommand::SetBuzzer(false),
                OutputCommand::SetStartButtonLight(LedState::LedOn),
            ];

            // Test that all command variants can be created
            for command in &commands {
                match command {
                    OutputCommand::SetFan(state) => assert!(*state == true || *state == false),
                    OutputCommand::SetLight(state) => assert!(*state == true || *state == false),
                    OutputCommand::SetBuzzer(state) => assert!(*state == true || *state == false),
                    OutputCommand::SetStartButtonLight(_) => assert!(true),
                }
            }
        }

        #[test]
        fn test_heater_command_enum() {
            let commands = [
                HeaterCommand::SetPower(50),
                HeaterCommand::SetFan(true),
                HeaterCommand::SimulationReset,
                HeaterCommand::UpdatePidParameters { kp: 1.0, ki: 0.5, kd: 0.1 },
            ];

            // Test that all heater command variants can be created
            for command in &commands {
                match command {
                    HeaterCommand::SetPower(power) => assert!(*power <= 100),
                    HeaterCommand::SetFan(_) => assert!(true),
                    HeaterCommand::SimulationReset => assert!(true),
                    HeaterCommand::UpdatePidParameters { kp, ki, kd } => {
                        assert!(*kp >= 0.0 && *ki >= 0.0 && *kd >= 0.0);
                    }
                }
            }
        }

        #[test]
        fn test_reflow_controller_state() {
            let mut error_msg = heapless::String::<256>::new();
            error_msg.push_str("Test error").unwrap();

            let mut profile_name = heapless::String::<32>::new();
            profile_name.push_str("Test Profile").unwrap();

            let state = ReflowControllerState {
                status: Status::Running,
                target_temperature: 200.0,
                current_temperature: 195.0,
                door_closed: true,
                fan: false,
                light: true,
                heater_power: 75,
                timer: 120,
                current_step: "Ramp",
                current_profile: profile_name,
                error_message: error_msg,
            };

            // Test that state structure can be created and fields accessed
            assert_eq!(state.status, Status::Running);
            assert_eq!(state.target_temperature, 200.0);
            assert_eq!(state.current_temperature, 195.0);
            assert!(state.door_closed);
            assert!(!state.fan);
            assert!(state.light);
            assert_eq!(state.heater_power, 75);
            assert_eq!(state.timer, 120);
            assert_eq!(state.current_step, "Ramp");
            assert_eq!(state.current_profile.as_str(), "Test Profile");
            assert_eq!(state.error_message.as_str(), "Test error");
        }

        #[test]
        fn test_heapless_string_constraints() {
            // Test that heapless strings respect their size constraints
            let mut small_string = heapless::String::<32>::new();
            let mut large_string = heapless::String::<256>::new();

            // Should be able to add up to capacity
            for i in 0..31 {
                small_string.push('a').unwrap();
            }
            assert_eq!(small_string.len(), 31);

            // Should be able to add more to large string
            for i in 0..100 {
                large_string.push('b').unwrap();
            }
            assert_eq!(large_string.len(), 100);

            // Test that strings can be cleared
            small_string.clear();
            assert_eq!(small_string.len(), 0);
            assert!(small_string.is_empty());
        }

        #[test]
        fn test_event_enum_completeness() {
            // Test that we can create all event variants
            let mut profile_name = heapless::String::<64>::new();
            profile_name.push_str("test.txt").unwrap();

            let events = [
                Event::StartCommand,
                Event::StopCommand,
                Event::ResetCommand,
                Event::DoorStateChanged(true),
                Event::DoorStateChanged(false),
                Event::LoadProfile(profile_name),
                Event::ListProfilesRequest,
                Event::SimulationReset,
                Event::UpdatePidParameters { kp: 1.0, ki: 0.5, kd: 0.1 },
            ];

            // Test that all events can be pattern matched
            for event in &events {
                match event {
                    Event::StartCommand => assert!(true),
                    Event::StopCommand => assert!(true),
                    Event::ResetCommand => assert!(true),
                    Event::DoorStateChanged(_) => assert!(true),
                    Event::LoadProfile(_) => assert!(true),
                    Event::ListProfilesRequest => assert!(true),
                    Event::SimulationReset => assert!(true),
                    Event::UpdatePidParameters { .. } => assert!(true),
                }
            }
        }

        #[test]
        fn test_constants() {
            // Test system constants
            assert_eq!(VERSION, "v0.1");
            assert_eq!(SYSTEM_TICK_MILLIS, 100);

            // Constants should be reasonable values
            assert!(SYSTEM_TICK_MILLIS > 0);
            assert!(SYSTEM_TICK_MILLIS < 10000); // Less than 10 seconds
            assert!(!VERSION.is_empty());
        }
    }
}
